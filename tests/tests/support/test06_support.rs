use crate::common::ConnExt;
use rusty_golf_actix::controller::db_prefill::db_prefill;
use rusty_golf_actix::model::{AllBettorScoresByRound, DetailedScore, SummaryDetailedScores};
use scraper::{Html, Selector};
use sql_middleware::middleware::{ConfigAndPool, DatabaseType, SqliteOptions};
use std::io::Write;
use std::path::Path;

pub(super) async fn setup_db() -> Result<ConfigAndPool, Box<dyn std::error::Error>> {
    let sqlite_options = SqliteOptions::new("file::memory:?cache=shared".to_string());
    let config_and_pool = ConfigAndPool::new_sqlite(sqlite_options).await?;
    let ddl = [
        include_str!("../../../actix/src/sql/schema/sqlite/00_event.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];
    let mut conn = config_and_pool.get_connection().await?;
    conn.execute_batch(&ddl.join("\n")).await?;
    let json = serde_json::from_str(include_str!("../test05_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;
    Ok(config_and_pool)
}

pub(super) async fn fetch_score_view_factor(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    conn.execute_dml(
        "UPDATE event SET refresh_from_espn = 1 WHERE espn_id = ?1;",
        &[sql_middleware::middleware::RowValues::Int(event_id.into())],
    )
    .await?;
    let result = conn
        .execute_select(
            "SELECT score_view_step_factor FROM event WHERE espn_id = ?1;",
            &[sql_middleware::middleware::RowValues::Int(event_id.into())],
        )
        .await?;
    let value = result.results[0]
        .get("score_view_step_factor")
        .and_then(sql_middleware::middleware::RowValues::as_float)
        .expect("Could not get score_view_step_factor from database");
    Ok(value.to_string().parse()?)
}

pub(super) fn assert_bar_widths_match(
    document: &Html,
    detailed_scores: &SummaryDetailedScores,
    step_factor: f32,
) {
    let selectors = ChartSelectors::new();
    for detailed_score in &detailed_scores.detailed_scores {
        let expected_label = expected_label_prefix(&detailed_score.golfer_name);
        for chart in document.select(&selectors.chart) {
            if chart.value().attr("data-player") != Some(detailed_score.bettor_name.as_str()) {
                continue;
            }
            assert_matching_chart_rows(
                chart,
                &selectors,
                detailed_score,
                &expected_label,
                step_factor,
            );
        }
    }
}

pub(super) fn save_debug_html(
    debug_path: &Path,
    html_output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(debug_path)?;
    let debug_file = debug_path.join("actual_output.html");
    let mut file = std::fs::File::create(&debug_file)?;
    writeln!(file, "{html_output}")?;
    Ok(())
}

pub(super) async fn test_render_template(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    _summary_scores_x: &AllBettorScoresByRound,
    _detailed_scores: &SummaryDetailedScores,
) -> Result<String, Box<dyn std::error::Error>> {
    let html = rusty_golf_actix::view::score::render_scores_template(
        &rusty_golf_actix::model::ScoreData {
            bettor_struct: vec![],
            score_struct: vec![],
            last_refresh: "1 minute".to_string(),
            last_refresh_source: rusty_golf_actix::model::RefreshSource::Db,
            cache_hit: true,
        },
        true,
        config_and_pool,
        event_id,
    )
    .await?;
    Ok(extract_player_bar_section(&html.into_string()))
}

struct ChartSelectors {
    chart: Selector,
    chart_row: Selector,
    bar_label: Selector,
    bar: Selector,
}

impl ChartSelectors {
    fn new() -> Self {
        Self {
            chart: Selector::parse(".chart").unwrap(),
            chart_row: Selector::parse(".chart-row").unwrap(),
            bar_label: Selector::parse(".bar-label").unwrap(),
            bar: Selector::parse(".bar").unwrap(),
        }
    }
}

fn expected_label_prefix(golfer_name: &str) -> String {
    if golfer_name == "Min Woo Lee" {
        return "M. Woo ".to_string();
    }
    let golfer_name_parts: Vec<&str> = golfer_name.split_whitespace().collect();
    let golfer_first_initial = golfer_name_parts
        .first()
        .map_or(' ', |s| s.chars().next().unwrap_or(' '));
    let golfer_last_abbr = golfer_name_parts
        .last()
        .map_or("", |s| if s.len() > 5 { &s[0..5] } else { s });
    format!("{golfer_first_initial}. {golfer_last_abbr}")
}

fn assert_matching_chart_rows(
    chart: scraper::element_ref::ElementRef<'_>,
    selectors: &ChartSelectors,
    detailed_score: &DetailedScore,
    expected_label: &str,
    step_factor: f32,
) {
    for chart_row in chart.select(&selectors.chart_row) {
        let Some(label_element) = chart_row.select(&selectors.bar_label).next() else {
            continue;
        };
        let label_text = label_element.text().collect::<String>();
        if !label_text.starts_with(expected_label) {
            continue;
        }
        let bars: Vec<_> = chart_row.select(&selectors.bar).collect();
        assert_eq!(bars.len(), detailed_score.scores.len());
        for bar in &bars {
            if let Some((width_val, score)) = parse_bar_width_and_score(*bar) {
                let expected_width = expected_bar_width(score, step_factor);
                let diff = (width_val - expected_width).abs();
                assert!(
                    diff < 0.01,
                    "Bar width should be {expected_width}% but found {width_val}% for score {score} (diff: {diff})"
                );
            }
        }
    }
}

fn expected_bar_width(score: i32, step_factor: f32) -> f32 {
    if score == 0 {
        return step_factor;
    }
    score.abs().to_string().parse::<f32>().unwrap_or(0.0) * step_factor
}

fn parse_bar_width_and_score(bar: scraper::element_ref::ElementRef<'_>) -> Option<(f32, i32)> {
    let style = bar.value().attr("style")?;
    let width_str = style
        .split("width:")
        .nth(1)
        .and_then(|s| s.split(';').next())?
        .trim();
    if !width_str.ends_with('%') {
        return None;
    }
    let width_val = width_str.trim_end_matches('%').parse().ok()?;
    let bar_text: String = bar.text().collect();
    let score_str = bar_text.trim().replace("R1: ", "").replace("R2: ", "");
    let score = score_str.parse().ok()?;
    Some((width_val, score))
}

fn extract_player_bar_section(html_string: &str) -> String {
    let start_marker = "<h3 class=\"playerbars\">Score by Player</h3>";
    let end_marker = "<h3 class=\"playerbars\">Score by Golfer</h3>";
    if let (Some(start_idx), Some(end_idx)) =
        (html_string.find(start_marker), html_string.find(end_marker))
    {
        return html_string[start_idx..end_idx].to_string();
    }
    html_string.to_string()
}
