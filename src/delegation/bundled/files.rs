use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

pub fn directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "bundled helper directory is not a regular directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return directory(path);
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(error) => return Err(error.into()),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn data(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "bundled helper cache entry is not a regular file"
        );
        let stored = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path);
        if let Ok(mut file) = stored {
            let mut data = Vec::new();
            Read::by_ref(&mut file)
                .take(bytes.len() as u64 + 1)
                .read_to_end(&mut data)?;
            if Sha256::digest(data) == Sha256::digest(bytes) {
                file.set_permissions(fs::Permissions::from_mode(if executable {
                    0o500
                } else {
                    0o400
                }))?;
                return Ok(());
            }
        }
    }
    let mut staged = tempfile::NamedTempFile::new_in(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("invalid helper cache path"))?,
    )?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(if executable {
            0o500
        } else {
            0o400
        }))?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}
