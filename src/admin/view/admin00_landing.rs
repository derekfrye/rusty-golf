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
                div id="results" {}
                a href=(nn) {
                        "1. Check database tables"
                }
            }
        }
    }
}
