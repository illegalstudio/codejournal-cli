use anyhow::{Result, bail};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const LIMIT: u64 = 128 * 1024 * 1024;

pub fn extract(bytes: &[u8], directory: &Path) -> Result<PathBuf> {
    let path = directory.join(if cfg!(windows) { "cj.exe" } else { "cj" });
    #[cfg(not(windows))]
    {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(Cursor::new(bytes)));
        let mut found = false;
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.as_ref() != Path::new("cj") {
                continue;
            }
            if found || !entry.header().entry_type().is_file() || entry.size() > LIMIT {
                bail!("invalid executable entry in release archive");
            }
            copy(&mut entry, &path)?;
            found = true;
        }
        if !found {
            bail!("release archive does not contain cj");
        }
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
        if archive
            .file_names()
            .filter(|name| *name == "cj.exe")
            .count()
            != 1
        {
            bail!("release archive must contain exactly one cj.exe");
        }
        let mut entry = archive.by_name("cj.exe")?;
        if !entry.is_file() || entry.is_symlink() || entry.size() > LIMIT {
            bail!("invalid executable entry in release archive");
        }
        copy(&mut entry, &path)?;
    }
    Ok(path)
}

fn copy(reader: &mut impl Read, path: &Path) -> Result<()> {
    let mut output = File::create(path)?;
    if std::io::copy(&mut reader.take(LIMIT + 1), &mut output)? > LIMIT {
        bail!("unpacked release binary exceeds its size limit");
    }
    output.sync_all()?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn archive(kind: tar::EntryType) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(6);
        header.set_mode(0o755);
        header.set_entry_type(kind);
        if kind.is_symlink() {
            header.set_link_name("../outside").unwrap();
        }
        header.set_cksum();
        builder
            .append_data(&mut header, "cj", Cursor::new(b"binary"))
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn executable_is_extracted_but_links_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = extract(&archive(tar::EntryType::Regular), directory.path()).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"binary");
        assert!(extract(&archive(tar::EntryType::Symlink), directory.path()).is_err());
        assert!(extract(b"not an archive", directory.path()).is_err());
    }
}
