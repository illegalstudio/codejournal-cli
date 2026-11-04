use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

pub const LIMIT: usize = 2 * 1024 * 1024;

pub struct RollingLog {
    file: File,
    path: PathBuf,
    written: usize,
}

impl RollingLog {
    pub fn new(path: PathBuf) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let file = options.open(&path)?;
        Ok(Self {
            file,
            path,
            written: 0,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        if self.written + bytes.len() > LIMIT {
            let retained = (LIMIT / 2).min(self.written);
            self.file
                .seek(SeekFrom::Start((self.written - retained) as u64))?;
            let mut tail = Vec::with_capacity(retained);
            self.file.read_to_end(&mut tail)?;
            let mut replacement =
                tempfile::NamedTempFile::new_in(self.path.parent().context("invalid log path")?)?;
            replacement.write_all(&tail)?;
            self.file = replacement.persist(&self.path)?;
            self.written = tail.len();
            let marker = self.path.with_extension("truncated");
            match OpenOptions::new().create_new(true).write(true).open(marker) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.file.write_all(bytes)?;
        self.file.flush()?;
        self.written += bytes.len();
        Ok(())
    }
}
