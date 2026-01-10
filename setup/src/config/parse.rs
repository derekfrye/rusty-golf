use super::GolferByBettorInput;
use anyhow::{Context, Result, anyhow};

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
