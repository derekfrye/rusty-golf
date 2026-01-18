use super::GolferByBettorInput;
use anyhow::{Context, Result, anyhow};

pub(crate) struct ParseError {
    pub(crate) index: usize,
}

pub(crate) fn parse_auth_tokens(value: &str) -> Result<Vec<String>> {
    let tokens: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect();
    if tokens.is_empty() {
        return Err(anyhow!("auth tokens list is empty"));
    }
    for token in &tokens {
        if token.chars().count() < 8 {
            return Err(anyhow!("auth token must be at least 8 characters"));
        }
        if token.chars().any(char::is_control) {
            return Err(anyhow!("auth token contains non-printable characters"));
        }
    }
    Ok(tokens)
}

pub(crate) fn parse_golfers_by_bettor(value: &str) -> Result<Vec<GolferByBettorInput>> {
    let entries: Vec<GolferByBettorInput> =
        serde_json::from_str(value).context("parse golfers-by-bettor JSON")?;
    if entries.is_empty() {
        return Err(anyhow!("golfers-by-bettor list is empty"));
    }
    Ok(entries)
}

pub(crate) fn parse_event_ids(value: &str) -> Result<Vec<i64>> {
    let items = parse_items(value)
        .map_err(|err| anyhow!("invalid event id list: {}", format_parse_error(value, err.index)))?;
    if items.is_empty() {
        return Err(anyhow!("event id list is empty"));
    }
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let parsed = item
            .parse::<i64>()
            .with_context(|| format!("invalid event id: {item}"))?;
        ids.push(parsed);
    }
    Ok(ids)
}

pub(crate) fn parse_single_event_id(value: &str) -> Result<i64> {
    let ids = parse_event_ids(value)?;
    if ids.len() != 1 {
        return Err(anyhow!("--event-id requires a single event id"));
    }
    Ok(ids[0])
}

fn parse_items(input: &str) -> Result<Vec<String>, ParseError> {
    for (index, ch) in input.char_indices() {
        if ch.is_control() {
            return Err(ParseError { index });
        }
    }
    if !input.contains(['"', '\'']) {
        return Ok(split_items(input));
    }

    parse_items_with_quotes(input, true)
}

fn split_items(input: &str) -> Vec<String> {
    let normalized = input.replace(',', " ");
    normalized
        .split_whitespace()
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_items_with_quotes(input: &str, require_balanced: bool) -> Result<Vec<String>, ParseError> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut open_index = 0usize;
    for (index, ch) in input.char_indices() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            open_index = index;
            continue;
        }

        if ch == ',' || ch.is_whitespace() {
            if !current.is_empty() {
                items.push(current.clone());
                current.clear();
            }
            continue;
        }

        current.push(ch);
    }

    if quote.is_some() && require_balanced {
        return Err(ParseError { index: open_index });
    }

    if !current.is_empty() {
        items.push(current);
    }
    Ok(items)
}

fn format_parse_error(input: &str, index: usize) -> String {
    let mut caret_pos = 0usize;
    for (byte_idx, _) in input.char_indices() {
        if byte_idx >= index {
            break;
        }
        caret_pos += 1;
    }
    let mut marker = String::new();
    marker.push_str(&" ".repeat(caret_pos));
    marker.push('^');
    format!(
        "Invalid character at position {}:\n{}\n{}",
        caret_pos + 1,
        input,
        marker
    )
}
