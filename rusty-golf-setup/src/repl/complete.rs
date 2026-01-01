use crate::repl::parse::split_items;
use rustyline::completion::Pair;
use std::collections::BTreeSet;

pub(crate) fn complete_items_prompt(
    line: &str,
    pos: usize,
    ids: &[String],
) -> (usize, Vec<Pair>) {
    let prefix = &line[..pos];
    let selected = split_items(prefix);
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
    if current_token.is_empty() {
        return (pos, candidates);
    }
    (start, candidates)
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
