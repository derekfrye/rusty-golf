use maud::{Markup, html};

use crate::HTMX_PATH;

#[must_use]
pub fn render_index_template(title: &str) -> Markup {
    html! {
        (maud::DOCTYPE)
        head{
            meta charset="UTF-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="stylesheet" type="text/css" href="static/styles.css";
            title { (title) }
            script src=(HTMX_PATH) defer {}
            script src="static/tablesort.js" defer {}
            script src="static/params.js" defer {}
            script src="static/scores.js" defer {}
        }
        body {
            h1 {
                (title)
            }
            div id="scores" {
                img alt="Result loading..." class="htmx-indicator" width="150" src="https://htmx.org//img/bars.svg" {}
            }
        }
    }
}
