use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let result = Command::new("git").args(args).output().ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}

fn main() {
    println!("cargo:rerun-if-env-changed=CJ_BUILD_REVISION");
    println!("cargo:rerun-if-env-changed=CJ_BUILD_DIRTY");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(common) = git(&["rev-parse", "--git-common-dir"]) {
        println!("cargo:rerun-if-changed={common}/refs");
        println!("cargo:rerun-if-changed={common}/packed-refs");
    }
    let revision = std::env::var("CJ_BUILD_REVISION")
        .ok()
        .or_else(|| git(&["rev-parse", "--short=12", "HEAD"]))
        .filter(|value| {
            (7..=40).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = std::env::var("CJ_BUILD_DIRTY")
        .ok()
        .map(|value| value == "1")
        .unwrap_or_else(|| {
            git(&[
                "status",
                "--porcelain",
                "--untracked-files=normal",
                "--",
                ".",
            ])
            .is_some_and(|status| !status.is_empty())
        });
    println!("cargo:rustc-env=CJ_BUILD_REVISION={revision}");
    println!(
        "cargo:rustc-env=CJ_BUILD_SUFFIX={}",
        if dirty { "+dirty" } else { "" }
    );
}
