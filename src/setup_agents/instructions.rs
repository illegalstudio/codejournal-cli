use super::files;
use anyhow::{Result, bail};
use std::fs;
use std::path::Path;

const START: &str = "<!-- code-journal:start -->";
const END: &str = "<!-- code-journal:end -->";

pub fn installed(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|body| body.contains(START) && body.contains(END))
}

pub fn apply(path: &Path, skill: &Path, uninstall: bool, dry_run: bool) -> Result<bool> {
    let before = match fs::read_to_string(path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let block = format!(
        "{START}\n## Code Journal\n\nRead `{}` before using Code Journal. Run `cj brief` at the start of each repository session and follow its project rules. Search with `cj search` before repeating an investigation. Record durable discoveries with `cj add` and completed work with `cj log add`. Never save credentials or personal data in the journal.\n{END}",
        skill.display()
    );
    let after = replace(&before, if uninstall { None } else { Some(&block) })?;
    if before == after {
        return Ok(false);
    }
    if !dry_run {
        if after.is_empty() {
            fs::remove_file(path)?;
        } else {
            files::write(path, &after)?;
        }
    }
    Ok(true)
}

fn replace(before: &str, block: Option<&str>) -> Result<String> {
    if before.matches(START).count() > 1 || before.matches(END).count() > 1 {
        bail!("duplicate Code Journal instruction markers; fix them before setup");
    }
    match (before.find(START), before.find(END)) {
        (Some(start), Some(end)) if start < end => {
            let end = end + END.len();
            let prefix = &before[..start];
            let suffix = &before[end..];
            if let Some(block) = block {
                Ok(format!("{prefix}{block}{suffix}"))
            } else {
                // Setup adds a separator after existing content. Remove only that separator.
                let prefix = prefix.strip_suffix("\n\n").unwrap_or(prefix);
                let suffix = suffix.strip_prefix('\n').unwrap_or(suffix);
                Ok(format!("{prefix}{suffix}"))
            }
        }
        (None, None) => Ok(match block {
            Some(block) if before.is_empty() => format!("{block}\n"),
            Some(block) => format!("{before}\n\n{block}\n"),
            None => before.to_owned(),
        }),
        _ => bail!("incomplete Code Journal instruction markers; fix them before setup"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstall_preserves_user_content_exactly() {
        for body in ["", "Existing instructions", "Existing instructions\n"] {
            let installed = replace(body, Some(&format!("{START}\nmanaged\n{END}"))).unwrap();
            assert_eq!(replace(&installed, None).unwrap(), body);
        }
    }

    #[test]
    fn malformed_markers_are_never_overwritten() {
        assert!(replace(START, Some("new")).is_err());
        assert!(replace(&format!("{END}{START}"), None).is_err());
        assert!(replace(&format!("{START}{START}{END}"), None).is_err());
    }
}
