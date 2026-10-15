pub fn notices(params: &[(&str, String)], empty: bool) -> Vec<String> {
    let query = params
        .iter()
        .find(|(key, _)| *key == "q")
        .map_or("", |(_, value)| value.as_str());
    if !empty || query.trim().is_empty() {
        return vec![];
    }
    let mut notices = Vec::new();
    let words = query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|word| !word.is_empty());
    let phrase_or_any = params
        .iter()
        .any(|(key, value)| matches!(*key, "literal" | "whole_word" | "any") && value == "1");
    if !phrase_or_any && words.take(2).count() > 1 {
        notices.push("Search requires all terms by default. Retry with --any to match at least one term, keeping the same filters.".to_owned());
    }
    let scope = params
        .iter()
        .find(|(key, _)| *key == "project")
        .map_or(" --all-projects".to_owned(), |(_, slug)| {
            format!(" --local --project {}", shell_words::quote(slug))
        });
    notices.push(format!("Knowledge search covers entries. Search current documents with: cj doc list{scope} --grep {}", shell_words::quote(query)));
    notices
}
