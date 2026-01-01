use rustyline::completion::Pair;
use std::collections::BTreeSet;

pub(crate) fn complete_event_prompt(line: &str, pos: usize, ids: &[String]) -> (usize, Vec<Pair>) {
    let prefix = &line[..pos];
    if let Some(selected) = parse_event_ids(prefix) {
        let selected_set: BTreeSet<&str> = selected.iter().map(String::as_str).collect();
        let current_token = current_token_prefix(prefix);
        let candidates: Vec<Pair> = ids
            .iter()
            .filter(|id| !selected_set.contains(id.as_str()))
            .filter(|id| id.starts_with(current_token))
            .map(|id| Pair {
                display: id.clone(),
                replacement: id.clone(),
            })
            .collect();
        let start = token_start_index(prefix);
        return (start, candidates);
    }

    let candidates = ids
        .iter()
        .map(|id| Pair {
            display: id.clone(),
            replacement: id.clone(),
        })
        .collect();
    (pos, candidates)
}

fn parse_event_ids(input: &str) -> Option<Vec<String>> {
    let normalized = input.replace(',', " ");
    let mut ids = Vec::new();
    for token in normalized.split_whitespace() {
        if !token.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        ids.push(token.to_string());
    }
    Some(ids)
}

fn current_token_prefix(input: &str) -> &str {
    input
        .rsplit(|ch: char| ch.is_whitespace() || ch == ',')
        .next()
        .unwrap_or("")
}

fn token_start_index(input: &str) -> usize {
    input
        .rfind(|ch: char| ch.is_whitespace() || ch == ',')
        .map_or(0, |idx| idx + 1)
}
