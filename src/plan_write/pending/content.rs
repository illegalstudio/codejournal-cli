use anyhow::Result;

pub fn clean(body: &str, title: &str) -> Result<String> {
    let trim = |character: char| [' ', '\n', '\r', '\t', '\0', '\u{b}'].contains(&character);
    let mut body = body.trim_matches(trim);
    let heading = regex::Regex::new(r"\A#\s+([^\n]+)\n*")?;
    while let Some(found) = heading.captures(body) {
        if found[1].trim_matches(trim).to_lowercase() != title.trim_matches(trim).to_lowercase() {
            break;
        }
        if let Some(span) = found.get(0) {
            body = body[span.end()..].trim_start_matches(trim);
        }
    }
    Ok(body.to_owned())
}
