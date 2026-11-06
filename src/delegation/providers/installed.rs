mod query;

use anyhow::{Context, Result};
use serde_json::Value;
use std::{ffi::OsString, io::Read, path::Path, process::Command};

pub fn command(name: &str, session: &Value, dry: bool) -> Result<Command> {
    let mut command = Command::new(name);
    if dry {
        return Ok(command);
    }
    let Some(launcher) = super::find(name).filter(|path| is_mise(path)) else {
        return Ok(command);
    };
    let canonical = launcher
        .canonicalize()
        .context("cannot inspect mise launcher")?;
    let manager = if canonical.file_stem().is_some_and(|stem| stem == "mise") {
        canonical
    } else {
        super::find("mise").context("mise launcher found but mise is unavailable in PATH")?
    };
    let cwd = Path::new(super::super::text(session, "cwd"));
    let resolve = |tool: &str| {
        query::installed(&manager, tool, cwd)
            .filter(|path| !is_mise(path))
            .with_context(|| format!("mise launcher for {tool} has no resolvable installed executable; run `mise which {tool}` in {} and install or select it explicitly before delegating", cwd.display()))
    };
    let program = resolve(name)?;
    let mut directories = vec![
        program
            .parent()
            .context("provider has no installation directory")?
            .to_path_buf(),
    ];
    if let Some(interpreter) = interpreter(&program) {
        let adjacent = directories[0].join(&interpreter);
        let runtime = if adjacent.is_file() && !is_mise(&adjacent) {
            adjacent
        } else if let Some(runtime) = super::find(&interpreter).filter(|path| !is_mise(path)) {
            runtime
        } else {
            resolve(&interpreter)?
        };
        directories.push(
            runtime
                .parent()
                .context("interpreter has no installation directory")?
                .to_path_buf(),
        );
    }
    directories.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    command = Command::new(program);
    command.env("PATH", std::env::join_paths(directories)?);
    Ok(command)
}

fn is_mise(path: &Path) -> bool {
    let components = path
        .components()
        .map(|part| part.as_os_str())
        .collect::<Vec<_>>();
    let wrapper = components
        .windows(2)
        .any(|parts| parts == ["command-wrappers", "bin"])
        || components
            .windows(2)
            .any(|parts| parts == ["mise", "shims"]);
    wrapper
        || path
            .canonicalize()
            .ok()
            .and_then(|path| path.file_stem().map(OsString::from))
            .is_some_and(|stem| stem == "mise" || stem == "mise-shim")
}

fn interpreter(program: &Path) -> Option<String> {
    let mut text = String::new();
    std::fs::File::open(program)
        .ok()?
        .take(512)
        .read_to_string(&mut text)
        .ok()?;
    let line = text.lines().next()?.strip_prefix("#!/usr/bin/env ")?;
    let mut args = shell_words::split(line).ok()?.into_iter();
    let name = args.next()?;
    let name = if name == "-S" { args.next()? } else { name };
    (!name.starts_with('-') && !name.contains(['/', '='])).then_some(name)
}
