use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::Path;

const MAX_BYTES: u64 = 262_144;

pub fn read(path: &Path) -> Option<Vec<u8>> {
    if !fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= MAX_BYTES as usize).then_some(bytes)
}

pub fn text(path: &Path) -> Option<String> {
    String::from_utf8(read(path)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_bounded_regular_files() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("manifest");
        fs::write(&path, "module example.com/tool\n").unwrap();
        assert_eq!(text(&path).unwrap(), "module example.com/tool\n");
        fs::write(&path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        assert!(read(&path).is_none());
        assert!(read(folder.path()).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_links_devices_and_pipes_without_reading_them() {
        use std::os::unix::fs::symlink;
        let folder = tempfile::tempdir().unwrap();
        let link = folder.path().join("link");
        symlink("/dev/zero", &link).unwrap();
        assert!(read(&link).is_none());
        assert!(read(Path::new("/dev/zero")).is_none());
        let pipe = folder.path().join("pipe");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&pipe)
                .status()
                .unwrap()
                .success()
        );
        assert!(read(&pipe).is_none());
    }
}
