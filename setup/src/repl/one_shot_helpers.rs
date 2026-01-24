use crate::event_details::EventDetailsRow;
use anyhow::{Context, Result, anyhow};
use serde_json::to_string_pretty;
use std::fs;
use std::path::Path;

pub(super) fn match_golfer_exact(golfers: &[(String, i64)], golfer: &str) -> Result<i64> {
    if let Some((_, id)) = golfers.iter().find(|(name, _)| name == golfer) {
        return Ok(*id);
    }
    let suggestions = closest_golfers(golfers, golfer);
    if suggestions.is_empty() {
        return Err(anyhow!("Unknown golfer: {golfer}"));
    }
    Err(anyhow!(
        "Unknown golfer: {golfer}. Closest matches: {}",
        suggestions.join(", ")
    ))
}

pub(super) fn write_event_details(
    path: Option<&Path>,
    output_json_stdout: bool,
    rows: &[EventDetailsRow],
) -> Result<()> {
    let payload = to_string_pretty(rows).context("serialize event details")?;
    if output_json_stdout {
        println!("{payload}");
        return Ok(());
    }
    let Some(path) = path else {
        return Err(anyhow!("missing output path for event details"));
    };
    fs::write(path, payload).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn closest_golfers(golfers: &[(String, i64)], golfer: &str) -> Vec<String> {
    let target = golfer.to_lowercase();
    let mut ranked: Vec<(f64, &String)> = golfers
        .iter()
        .map(|(name, _)| (strsim::jaro_winkler(&name.to_lowercase(), &target), name))
        .collect();
    ranked.sort_by(|(score_a, name_a), (score_b, name_b)| {
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| name_a.cmp(name_b))
    });
    ranked
        .into_iter()
        .filter(|(score, _)| *score > 0.0)
        .take(3)
        .map(|(_, name)| name.clone())
        .collect()
}
