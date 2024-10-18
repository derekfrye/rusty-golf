use crate::model::db::test_is_db_setup;
// use crate::model::model_admin::{Bettor, Player};

// use actix_web::{web, HttpResponse};
use maud::{html, Markup};
// use std::collections::HashMap;
// use serde_json::json;

// Render the main page
pub async fn render_default_page() -> Markup {
    let qt = test_is_db_setup().await.unwrap();

    html! {
        p { (format!("{:?}",qt)) }
    }
}
