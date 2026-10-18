use anyhow::{Context, Result, ensure};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;

/// Canonicalize the broker's Unix seconds without changing the retained event on disk.
pub fn normalize(event: &mut Value) -> Result<()> {
    if event["type"] != "delegation" {
        return Ok(());
    }
    if let Some(value) = event.get_mut("ts") {
        timestamp(value, "ts")?;
    }
    if let Some(snapshot) = event.get_mut("delegation").and_then(Value::as_object_mut) {
        for field in ["created_at", "updated_at", "integrated_at"] {
            if let Some(value) = snapshot.get_mut(field) {
                timestamp(value, &format!("delegation.{field}"))?;
            }
        }
    }
    Ok(())
}

fn timestamp(value: &mut Value, field: &str) -> Result<()> {
    if !value.is_number() {
        return Ok(());
    }
    let unix = value
        .as_f64()
        .with_context(|| format!("Invalid Unix timestamp in {field}"))?;
    ensure!(
        (-62_135_596_800.0..253_402_300_800.0).contains(&unix),
        "Unix timestamp outside supported years 1-9999 in {field}"
    );
    let seconds = unix.floor() as i64;
    let nanos = ((unix - seconds as f64) * 1_000_000_000.0).round() as u32;
    let date = DateTime::<Utc>::from_timestamp(
        seconds + i64::from(nanos / 1_000_000_000),
        nanos % 1_000_000_000,
    )
    .with_context(|| format!("Invalid Unix timestamp in {field}"))?;
    *value = Value::String(date.to_rfc3339_opts(SecondsFormat::AutoSi, true));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use serde_json::json;

    #[test]
    fn broker_seconds_preserve_fractions_epoch_and_negative_dates() {
        let mut event = json!({"id":"original", "type":"delegation", "ts":0,
            "delegation":{"created_at":-0.25, "updated_at":0.125, "integrated_at":0}});
        normalize(&mut event).unwrap();
        assert_eq!(event["id"], "original");
        assert_eq!(event["ts"], "1970-01-01T00:00:00Z");
        assert_eq!(
            event["delegation"]["created_at"],
            "1969-12-31T23:59:59.750Z"
        );
        assert_eq!(
            event["delegation"]["updated_at"],
            "1970-01-01T00:00:00.125Z"
        );
        assert_eq!(event["delegation"]["integrated_at"], "1970-01-01T00:00:00Z");
    }

    #[test]
    fn dates_nulls_missing_fields_and_other_events_stay_unchanged() {
        for mut event in [
            json!({"type":"delegation", "ts":"2026-10-17T12:00:00+02:00",
                "delegation":{"created_at":"2026-10-17T10:00:00Z", "integrated_at":null}}),
            json!({"type":"start", "ts":12, "delegation":{"created_at":42}}),
            json!({"type":"delegation"}),
        ] {
            let before = event.clone();
            normalize(&mut event).unwrap();
            assert_eq!(event, before);
        }
    }

    #[test]
    fn out_of_range_dates_fail_without_fabricating_a_time() {
        for unix in [1e30, -62_135_596_801.0, 253_402_300_800.0] {
            let mut event = json!({"type":"delegation", "delegation":{"created_at":unix}});
            assert!(
                normalize(&mut event)
                    .unwrap_err()
                    .to_string()
                    .contains("delegation.created_at")
            );
        }
    }
}
