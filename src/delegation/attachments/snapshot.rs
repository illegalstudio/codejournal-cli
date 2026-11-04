use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::PathBuf};

pub fn capture(files: &[PathBuf], results: &[String]) -> Result<Value> {
    ensure!(
        files.len() + results.len() <= 20,
        "at most 20 context attachments are allowed"
    );
    let mut items = Vec::new();
    for (index, path) in files.iter().enumerate() {
        let mut bytes = Vec::new();
        File::open(path)?.take(1_000_001).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 1_000_000,
            "each context file must fit within 1000000 bytes"
        );
        let body = String::from_utf8(bytes).context("context files must be UTF-8")?;
        items.push(item(
            format!("file-{}", index + 1),
            json!({"path":path}),
            body,
        ));
    }
    for (index, id) in results.iter().enumerate() {
        let id = super::super::resolve::id(id)?;
        let session = super::super::store::get(&super::super::store::open()?, &id)?;
        ensure!(
            super::super::terminal(&session),
            "only terminal delegation results may be attached"
        );
        let body = session["result"]
            .as_str()
            .context("the previous delegation has no result")?
            .to_owned();
        items.push(item(
            format!("result-{}", index + 1),
            json!({"delegation_id":id,"attempt":session["attempt"]}),
            body,
        ));
    }
    ensure!(
        items
            .iter()
            .filter_map(|item| item["body"].as_str())
            .map(str::len)
            .sum::<usize>()
            <= 4_000_000,
        "context attachments exceed 4000000 bytes in total"
    );
    Ok(json!(items))
}

fn item(name: String, source: Value, body: String) -> Value {
    json!({"name":name,"source":source,"sha256":format!("{:x}",Sha256::digest(body.as_bytes())),"characters":body.chars().count(),"body":body})
}
