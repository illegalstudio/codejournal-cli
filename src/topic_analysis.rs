use serde_json::Value;

pub fn notice(data: &Value) -> Option<&'static str> {
    (data["topic_analysis"]["truncated"] == true).then_some(
        "Topic analysis is partial: the server reached its analysis budget. Some matches may be missing. Review the suggestions and use `cj topics merge SOURCES --into TARGET` for other known duplicates.",
    )
}

#[cfg(test)]
mod tests {
    use super::notice;
    use serde_json::json;

    #[test]
    fn incomplete_results_are_explicit_and_old_servers_remain_compatible() {
        assert!(notice(&json!({})).is_none());
        assert!(notice(&json!({"topic_analysis": {"truncated": false}})).is_none());
        assert!(
            notice(&json!({"topic_analysis": {"truncated": true}}))
                .unwrap()
                .contains("cj topics merge")
        );
    }
}
