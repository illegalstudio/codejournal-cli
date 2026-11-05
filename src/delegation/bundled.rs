#[cfg(target_os = "linux")]
mod files;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
include!(concat!(env!("OUT_DIR"), "/bundled_bwrap.rs"));

pub fn info() -> Value {
    json!({"available":BYTES.is_some(),"version":VERSION,"architecture":std::env::consts::ARCH,
        "sha256":BYTES.map(|bytes| format!("{:x}", Sha256::digest(bytes)))})
}

#[cfg(target_os = "linux")]
pub fn extract(dry: bool) -> anyhow::Result<std::path::PathBuf> {
    use anyhow::Context;
    let bytes = BYTES.context("this source build has no bundled Bubblewrap helper")?;
    if dry {
        return Ok("<bundled bubblewrap>".into());
    }
    let state = super::store::directory()?
        .parent()
        .context("invalid runtime directory")?
        .canonicalize()?;
    let digest = format!("{:x}", Sha256::digest(bytes));
    let root = state.join("helpers");
    files::directory(&root)?;
    let root = root.join(format!("bubblewrap-{VERSION}-{digest}"));
    files::directory(&root)?;
    let notices: [(&str, &[u8]); 5] = [
        (
            "COPYING.bubblewrap",
            include_bytes!("../../licenses/bubblewrap.txt"),
        ),
        (
            "COPYING.libcap",
            include_bytes!("../../licenses/libcap.txt"),
        ),
        ("COPYING.musl", include_bytes!("../../licenses/musl.txt")),
        (
            "COPYING.gcc-runtime",
            include_bytes!("../../licenses/gcc-runtime-exception.txt"),
        ),
        (
            "COPYING.gcc-gpl",
            include_bytes!("../../licenses/gcc-gpl-3.0.txt"),
        ),
    ];
    for (name, data) in notices {
        files::data(&root.join(name), data, false)?;
    }
    let path = root.join("bwrap");
    files::data(&path, bytes, true)?;
    Ok(path)
}
