use anyhow::{Context, Result, bail};

pub fn step(body: &str, index: u64, done: bool) -> Result<String> {
    let fence_pattern = regex::Regex::new(r"^ {0,3}(`{3,}|~{3,})(.*)$")?;
    let item_pattern = regex::Regex::new(r"^(\s*[-+*] \[)[ xX](\] .*)$")?;
    let mut fence: Option<(u8, usize)> = None;
    let mut number = 0;
    let mut lines = body.split('\n').map(str::to_owned).collect::<Vec<_>>();
    for line in &mut lines {
        if let Some(marker) = fence_pattern.captures(line) {
            let span = marker.get(1).context("missing fence")?.as_str();
            let value = (span.as_bytes()[0], span.len());
            match fence {
                None => fence = Some(value),
                Some((character, length))
                    if character == value.0
                        && value.1 >= length
                        && marker
                            .get(2)
                            .is_some_and(|text| text.as_str().trim().is_empty()) =>
                {
                    fence = None
                }
                _ => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if let Some(item) = item_pattern.captures(line) {
            number += 1;
            if number == index {
                *line = format!("{}{}{}", &item[1], if done { "x" } else { " " }, &item[2]);
                return Ok(lines.join("\n"));
            }
        }
    }
    bail!("Checklist item not found in local pending content")
}
