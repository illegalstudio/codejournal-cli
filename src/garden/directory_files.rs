use anyhow::{Context, Result, ensure};
use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
};

/// Merge Git's sorted tracked and untracked streams, keeping only a few filenames in memory.
pub fn visit(root: &Path, path: &str, mut consume: impl FnMut(&[u8]) -> Result<()>) -> Result<()> {
    let mut tracked = GitFiles::start(root, path, "--cached")?;
    let mut untracked = GitFiles::start(root, path, "--others")?;
    let mut left = tracked.next()?;
    let mut right = untracked.next()?;
    let mut previous = None;
    while left.is_some() || right.is_some() {
        let first = match (&left, &right) {
            (Some(a), Some(b)) => a <= b,
            (Some(_), None) => true,
            _ => false,
        };
        let (source, cursor) = if first {
            (&mut tracked, &mut left)
        } else {
            (&mut untracked, &mut right)
        };
        if let Some(name) = cursor.take() {
            if previous.as_ref() != Some(&name) {
                consume(&name)?;
                previous = Some(name);
            }
            *cursor = source.next()?;
        }
    }
    Ok(())
}

struct GitFiles {
    child: Child,
    output: BufReader<ChildStdout>,
    finished: bool,
}

impl GitFiles {
    fn start(root: &Path, path: &str, kind: &str) -> Result<Self> {
        let mut child = Command::new("git")
            .current_dir(root)
            .args([
                "--literal-pathspecs",
                "ls-files",
                "--full-name",
                "-z",
                kind,
                "--exclude-standard",
                "--",
                path,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Cannot start Git to list garden reference files")?;
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Git garden file listing has no output pipe");
        };
        Ok(Self {
            child,
            output: BufReader::new(stdout),
            finished: false,
        })
    }

    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        if self.finished {
            return Ok(None);
        }
        let mut name = Vec::new();
        if self.output.read_until(0, &mut name)? == 0 {
            let status = self.child.wait()?;
            self.finished = true;
            ensure!(status.success(), "Git garden file listing failed: {status}");
            return Ok(None);
        }
        ensure!(name.pop() == Some(0), "Incomplete Git garden filename");
        Ok(Some(name))
    }
}

impl Drop for GitFiles {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
