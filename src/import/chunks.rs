use anyhow::{Result, bail};
use serde_json::Value;

// Reserve space for JSONB separators and redaction expansion on the server.
const MAX_BYTES: usize = 524_288;

pub fn build(records: Vec<Value>) -> Result<Vec<Vec<Value>>> {
    let mut chunks = Vec::new();
    let mut chunk = Vec::new();
    let mut bytes = 2;
    for record in records {
        let size = serde_json::to_vec(&record)?.len() + 1;
        if size + 2 > MAX_BYTES {
            bail!(
                "import record exceeds the 512 KiB client chunk budget; split the record before importing"
            );
        }
        if chunk.len() == 100 || bytes + size > MAX_BYTES {
            chunks.push(std::mem::take(&mut chunk));
            bytes = 2;
        }
        bytes += size;
        chunk.push(record);
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unicode_records_are_packed_by_bytes_and_count() {
        let records = vec![json!({"body": "😀".repeat(80_000)}); 3];
        let chunks = build(records).unwrap();
        assert_eq!(chunks.len(), 3);
        assert!(
            chunks
                .iter()
                .all(|chunk| serde_json::to_vec(chunk).unwrap().len() <= MAX_BYTES)
        );
        assert_eq!(build(vec![json!({}); 101]).unwrap().len(), 2);
        assert!(build(vec![json!({"body": "x".repeat(MAX_BYTES)})]).is_err());
        assert!(build(vec![]).unwrap().is_empty());
    }
}
