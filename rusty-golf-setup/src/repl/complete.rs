use crate::repl::parse::split_items_relaxed;
use rustyline::completion::Pair;
use std::collections::BTreeSet;

pub(crate) fn complete_items_prompt(
    line: &str,
    pos: usize,
    items: &[String],
    quote_items: bool,
) -> (usize, Vec<Pair>) {
    let prefix = &line[..pos];
    let selected = split_items_relaxed(prefix);
    let selected_set: BTreeSet<&str> = selected.iter().map(String::as_str).collect();
    let current_token = current_token_prefix(prefix);
    let match_token = current_token.trim_start_matches(['"', '\'']);
    let candidates: Vec<Pair> = items
        .iter()
        .filter(|item| !selected_set.contains(item.as_str()))
        .filter(|item| matches_token(item, match_token))
        .map(|item| Pair {
            display: item.clone(),
            replacement: format_completion(item, quote_items),
        })
        .collect();
    let start = token_start_index(prefix);
    if current_token.is_empty() {
        return (pos, candidates);
    }
    (start, candidates)
}

fn matches_token(item: &str, token: &str) -> bool {
    if token.is_empty() {
        return true;
    }
    let token = token.to_lowercase();
    let item_lc = item.to_lowercase();
    if item_lc.starts_with(&token) {
        return true;
    }
    item_lc
        .split_whitespace()
        .any(|part| part.starts_with(&token))
}

fn format_completion(item: &str, quote_items: bool) -> String {
    if quote_items {
        format!("\"{}\"", item)
    } else {
        item.to_string()
    }
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
