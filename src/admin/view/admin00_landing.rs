use crate::{
    admin::model::admin_model::{AdminPage, AlphaNum14},
    HTMX_PATH,
};

use maud::{html, Markup};

// Render the main page
pub async fn render_default_page(token: AlphaNum14) -> Markup {
    let next_page = AdminPage::get_page_number(&AdminPage::TablesAndConstraints);
    let nn = format!("admin?p={}&token={}", next_page, token.value());

    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Golf Admin Setup Page" }
                // Include htmx
                script src=(HTMX_PATH) defer {}
                link rel="preload" href="static/styles.css" as="style" onload="this.rel='stylesheet'";
            }
            body {
                div class="grid-container" {
                    div id="landing_grid_1" {
                        div id="cell_header_1" class="cell-header" { "TABLES" }
                        div id="cell_body_1" class="cell-body" {"Checking..."}
                    }
                    div id="landing_grid_2" {
                        div id="cell_header_2" class="cell-header" { "CONSTRAINTS" }
                        div id="cell_body_2" class="cell-body" {"Checking..."}
                    }
                    div id="landing_grid_3" {
                        div id="cell_header_3" class="cell-header" { "FUNCTIONS" }
                        div id="cell_body_3" class="cell-body" {"Checking..."}
                    }
                    div id="landing_grid_4" {
                        div id="cell_header_4" class="cell-header" { "SOMETHING" }
                        div id="cell_body_4" class="cell-body" {"Checking..."}
                    }
                }
                div id="results" {}
                a href=(nn) {
                        "1. Check database tables"
                }
                script src="static/admin_landing.js" defer {}
            }
        }
    }
}
