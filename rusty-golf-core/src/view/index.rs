use maud::{Markup, html};

use crate::HTMX_PATH;

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
            script src=(HTMX_PATH) defer {}
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
