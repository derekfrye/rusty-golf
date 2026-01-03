use crate::repl::parse::split_items_relaxed;
use rustyline::completion::Pair;
use std::collections::BTreeSet;
use std::path::PathBuf;

pub(crate) fn complete_items_prompt(
    line: &str,
    pos: usize,
    items: &[String],
    quote_items: bool,
) -> (usize, Vec<Pair>) {
    complete_prompt(line, pos, |selected_set, current_token| {
        let match_token = current_token.trim_start_matches(['"', '\'']);
        items
            .iter()
            .filter(|item| !selected_set.contains(item.as_str()))
            .filter(|item| matches_token(item, match_token))
            .map(|item| Pair {
                display: item.clone(),
                replacement: format_completion(item, quote_items),
            })
            .collect()
    })
}

pub(crate) fn complete_path_prompt(
    line: &str,
    pos: usize,
    paths: &[PathBuf],
) -> (usize, Vec<Pair>) {
    complete_prompt(line, pos, |selected_set, current_token| {
        let match_token = current_token
            .trim_start_matches(['"', '\''])
            .to_lowercase();
        paths
            .iter()
            .filter_map(|path| {
                let display = path.display().to_string();
                let file_name = path.file_name()?.to_string_lossy();
                if selected_set.contains(display.as_str()) {
                    return None;
                }
                if !match_path_token(&display, &file_name, &match_token) {
                    return None;
                }
                Some(Pair {
                    display: display.clone(),
                    replacement: display,
                })
            })
            .collect()
    })
}

fn complete_prompt<F>(line: &str, pos: usize, mut build: F) -> (usize, Vec<Pair>)
where
    F: FnMut(&BTreeSet<&str>, &str) -> Vec<Pair>,
{
    let prefix = &line[..pos];
    let selected = split_items_relaxed(prefix);
    let selected_set: BTreeSet<&str> = selected.iter().map(String::as_str).collect();
    let current_token = current_token_prefix(prefix);
    let candidates = build(&selected_set, current_token);
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
        format!("\"{item}\"")
    } else {
        item.to_string()
    }
}

fn match_path_token(display: &str, file_name: &str, token: &str) -> bool {
    if token.is_empty() {
        return true;
    }
    let display_lc = display.to_lowercase();
    let file_lc = file_name.to_lowercase();
    display_lc.starts_with(token) || file_lc.starts_with(token)
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
