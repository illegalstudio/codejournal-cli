use super::super::store;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::Child,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub struct Streams(Vec<JoinHandle<()>>);

impl Streams {
    pub fn complete(self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.0.iter().any(|h| !h.is_finished()) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        for handle in self.0 {
            if handle.is_finished() {
                let _ = handle.join();
            }
        }
    }
}

fn path(id: &str, session: &Value, name: &str) -> Result<PathBuf> {
    Ok(store::directory()?.join(format!("{id}-attempt-{}.{name}.log", session["attempt"])))
}

pub fn start(id: &str, session: &Value, child: &mut Child) -> Result<Streams> {
    let activity = Arc::new(AtomicU64::new(0));
    let stdout = child.stdout.take().context("provider stdout unavailable")?;
    let stderr = child.stderr.take().context("provider stderr unavailable")?;
    Ok(Streams(vec![
        capture(stdout, path(id, session, "stdout")?, id, activity.clone())?,
        capture(stderr, path(id, session, "stderr")?, id, activity)?,
    ]))
}

fn capture(
    mut input: impl Read + Send + 'static,
    path: PathBuf,
    id: &str,
    activity: Arc<AtomicU64>,
) -> Result<JoinHandle<()>> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    let id = id.to_owned();
    Ok(std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        let mut written = 0;
        while let Ok(count) = input.read(&mut buffer) {
            if count == 0 {
                break;
            }
            let amount = count.min((2 * 1024 * 1024usize).saturating_sub(written));
            if amount > 0 {
                if file.write_all(&buffer[..amount]).is_err() {
                    break;
                }
                let _ = file.flush();
                written += amount;
            }
            let now = super::super::now() as u64;
            if activity.swap(now, Ordering::Relaxed) != now {
                let _ = store::transaction(|db| {
                    let session = store::get(db, &id)?;
                    super::super::context::authorize(&session, "guest")?;
                    if !super::super::terminal(&session) {
                        store::update(
                            db,
                            &id,
                            json!({"last_activity":super::super::now(),"guest_heartbeat":super::super::now()}),
                        )?;
                    }
                    Ok(())
                });
            }
        }
    }))
}

pub fn tails(id: &str, session: &Value) -> Result<(String, String)> {
    Ok((
        tail(path(id, session, "stdout")?)?,
        tail(path(id, session, "stderr")?)?,
    ))
}

fn tail(path: PathBuf) -> Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    file.seek(SeekFrom::Start(size.saturating_sub(16384)))?;
    let mut bytes = Vec::new();
    file.take(16384).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
