use maud::{Markup, html};

use crate::error::CoreError;
use crate::storage::Storage;
use crate::HTMX_PATH;

pub const DEFAULT_INDEX_TITLE: &str = "Scoreboard";

/// Resolve an event-specific title from query input.
///
/// # Errors
/// Returns an error if parsing the event id fails or the storage lookup fails.
pub async fn try_resolve_index_title(
    storage: &dyn Storage,
    event_str: &str,
) -> Result<String, CoreError> {
    let event_id = event_str
        .trim()
        .parse::<i32>()
        .map_err(|e| CoreError::Parse(e.to_string()))?;
    let event_details = storage.get_event_details(event_id).await?;
    Ok(event_details.event_name)
}

pub async fn resolve_index_title_or_default(
    storage: &dyn Storage,
    event_str: &str,
) -> String {
    match try_resolve_index_title(storage, event_str).await {
        Ok(title) => title,
        Err(_) => DEFAULT_INDEX_TITLE.to_string(),
    }
}

#[must_use]
pub fn render_index_template(title: &str) -> Markup {
    html! {
        (maud::DOCTYPE)
        head{
            meta charset="UTF-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link id="theme-stylesheet" rel="stylesheet" type="text/css" href="static/alt/zen218.v7.css" data-theme-new="static/alt/zen218.v7.css" data-theme-classic="static/styles.v2.css";
            link rel="stylesheet" href="static/ex.css";
            title { (title) }
            script src=(HTMX_PATH) defer integrity="sha384-/TgkGk7p307TH7EXJDuUlgG3Ce1UVolAOFopFekQkkXihi5u/6OCvVKyz1W+idaz" crossorigin="anonymous" {}
            script src="static/tablesort.js" defer {}
            script src="static/params.js" defer {}
            script src="static/scores.js" defer {}
            script src="static/ex.js" defer {}
        }
        body {
            div class="switches" {
                button class="theme-toggle" id="theme-toggle" title="Toggles classic & new" aria-label="auto" aria-live="polite" {
                    span class="theme-label" { "Theme:" }
                    span class="theme-toggle-text theme-toggle-classic" { "classic" }
                    span class="theme-toggle-text theme-toggle-text-new" { "new" }
                }
            }
            h1 {
                (title)
            }
            div id="scores" {
                img alt="Result loading..." class="htmx-indicator" width="150" src="https://htmx.org//img/bars.svg" {}
            }
        }
    }
}
