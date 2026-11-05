use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

pub fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(env::var("OUT_DIR")?);
    let arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    let source = PathBuf::from("target/bundled-bwrap")
        .join(&arch)
        .join("bwrap");
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        source.with_file_name("provenance.json").display()
    );
    let pinned: serde_json::Value =
        serde_json::from_slice(&fs::read("scripts/sandbox/sources.json")?)?;
    println!("cargo:rerun-if-changed=scripts/sandbox/sources.json");
    let version = pinned["bubblewrap"]["version"]
        .as_str()
        .ok_or("missing helper version")?;
    let enabled = env::var("CARGO_CFG_TARGET_OS")? == "linux" && source.is_file();
    let content = if enabled {
        let bytes = fs::read(&source)?;
        let machine = if arch == "x86_64" {
            62_u16
        } else if arch == "aarch64" {
            183_u16
        } else {
            return Err("unsupported bundled helper architecture".into());
        };
        if bytes.len() < 64
            || bytes[..6] != *b"\x7fELF\x02\x01"
            || u16::from_le_bytes([bytes[18], bytes[19]]) != machine
        {
            return Err("bundled helper ELF architecture mismatch".into());
        }
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(source.with_file_name("provenance.json"))?)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let mut recipe = Sha256::new();
        for name in ["build.py", "container.sh", "sources.json"] {
            let path = format!("scripts/sandbox/{name}");
            println!("cargo:rerun-if-changed={path}");
            recipe.update(fs::read(path)?);
        }
        if metadata["architecture"] != arch
            || metadata["binary_sha256"] != digest
            || metadata["bubblewrap"] != pinned["bubblewrap"]
            || metadata["libcap"] != pinned["libcap"]
            || metadata["builder_image"] != pinned["builder_image"]
            || metadata["build_recipe_sha256"] != format!("{:x}", recipe.finalize())
        {
            return Err(
                "bundled helper provenance mismatch; rebuild with scripts/sandbox/build.py".into(),
            );
        }
        let offset = usize::try_from(u64::from_le_bytes(bytes[32..40].try_into()?))?;
        let stride = usize::from(u16::from_le_bytes(bytes[54..56].try_into()?));
        let count = usize::from(u16::from_le_bytes(bytes[56..58].try_into()?));
        for index in 0..count {
            let start = offset
                .checked_add(index.checked_mul(stride).ok_or("invalid ELF headers")?)
                .ok_or("invalid ELF headers")?;
            let header = bytes
                .get(start..start.checked_add(4).ok_or("invalid ELF headers")?)
                .ok_or("invalid ELF headers")?;
            if u32::from_le_bytes(header.try_into()?) == 3 {
                return Err("bundled helper requires a dynamic loader".into());
            }
        }
        let helper = output.join("bundled-bwrap");
        fs::write(&helper, bytes)?;
        format!(
            "const BYTES: Option<&[u8]> = Some(include_bytes!({:?}));\n",
            helper.to_string_lossy()
        )
    } else {
        "const BYTES: Option<&[u8]> = None;\n".to_owned()
    };
    fs::write(
        output.join("bundled_bwrap.rs"),
        format!("const VERSION: &str = {version:?};\n{content}"),
    )?;
    Ok(())
}
