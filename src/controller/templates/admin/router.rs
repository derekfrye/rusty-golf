use crate::model::admin_model::{AdminPage, AlphaNum14};

use actix_web::{web, HttpResponse};
use std::{collections::HashMap, env};

use super::admin01_tables::get_html_for_create_tables;

pub async fn router(query: web::Query<HashMap<String, String>>) -> HttpResponse {
    let token_str = query
        .get("token")
        .unwrap_or(&String::new())
        .trim()
        .to_string();

    // let mut token: AlphaNum14 = AlphaNum14::default();
    let token: AlphaNum14 = match AlphaNum14::parse(&token_str) {
        Ok(id) => id,
        Err(_) => AlphaNum14::default(),
    };
    let admin_page = AdminPage::parse(
        query
            .get("p")
            .unwrap_or(&String::new())
            .trim()
            .to_string()
            .as_str(),
    );

    // the token determines authorized access or not
    // see README if you're trying to figure out how to set this
    if let Ok(env_token) = env::var("TOKEN") {
        // unauthorized
        if env_token != token.value() {
            return HttpResponse::Unauthorized()
                .content_type("text/html; charset=utf-8")
                .body(UNAUTHORIZED_BODY);
        }
    }

    if admin_page == AdminPage::TablesAndConstraints {
        if query.contains_key("admin01_missing_tables") {
            get_html_for_create_tables(query).await
        } else {
            // missing_tables is populated by admin.js, so when empty it means user browsed to this admin page rather than js submitting to it
            let x =
                crate::controller::templates::admin::admin01_tables::render_default_page().await;
            HttpResponse::Ok()
                .content_type("text/html")
                .body(x.into_string())
        }
    } else if admin_page == AdminPage::ZeroX {
        if query.contains_key("data") {
            let markup_from_admin =
                crate::controller::templates::admin::admin0x::display_received_data(query);
            HttpResponse::Ok()
                .content_type("text/html")
                .body(markup_from_admin.into_string())
        } else {
            let markup = crate::controller::templates::admin::admin0x::render_default_page().await;

            HttpResponse::Ok()
                .content_type("text/html")
                .body(markup.into_string())
        }
    } else if admin_page == AdminPage::Landing {
        let markup =
            crate::controller::templates::admin::admin00_landing::render_default_page(token).await;
        HttpResponse::Ok()
            .content_type("text/html")
            .body(markup.into_string())
    } else {
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(INVALID_ADMIN_BODY)
    }
}

const UNAUTHORIZED_BODY: &str = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Unauthorized</title>
        <style>
            body { font-family: Arial, sans-serif; background-color: #f4f4f4; text-align: center; padding: 50px; }
            h1 { color: #333; }
            p { color: #666; }
        </style>
    </head>
    <body>
        <h1>401 Unauthorized</h1>
    </body>
    </html>
    "#;

const INVALID_ADMIN_BODY: &str = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Invalid page</title>
        <style>
            body { font-family: Arial, sans-serif; background-color: #f4f4f4; text-align: center; padding: 50px; }
            h1 { color: #333; }
            p { color: #666; }
        </style>
    </head>
    <body>
        <h1>Sorry, we can't find that admin page.</h1>
        <p>Check your <pre>p</pre> parameter.</p>
    </body>
    </html>
    "#;
