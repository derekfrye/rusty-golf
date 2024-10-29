use crate::model::admin_model::AlphaNum14;

use actix_web::{ web, HttpResponse };
use std::{ collections::HashMap, env };

use super::admin00::get_html_for_create_tables;

const UNAUTHORIZED_BODY: &str =
    r#"
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

pub async fn router(query: web::Query<HashMap<String, String>>) -> HttpResponse {
    let token_str = query.get("token").unwrap_or(&String::new()).trim().to_string();

    // let mut token: AlphaNum14 = AlphaNum14::default();
    let token: AlphaNum14 = match AlphaNum14::parse(&token_str) {
        Ok(id) => id,
        Err(_) => AlphaNum14::default(),
    };
    let p = query.get("p").unwrap_or(&String::new()).trim().to_string();

    // check query string token against the token this container was started with
    match env::var("TOKEN") {
        Ok(env_token) => {
            // unauthorized
            if env_token != token.value() {
                return HttpResponse::Unauthorized()
                    .content_type("text/html; charset=utf-8")
                    .body(UNAUTHORIZED_BODY);
            } else {
                // authorized
                // missing_tables is populated by admin.js, so when empty it means user browsed to this admin page rather than js submitting to it
                // display admin00
                if p == "00" {
                    if query.contains_key("admin00_missing_tables") {
                        get_html_for_create_tables(query).await
                    } else {
                        let x =
                            crate::controller::templates::admin::admin00::render_default_page().await;
                        HttpResponse::Ok().content_type("text/html").body(x.into_string())
                    }
                    //
                    // admin0x, render the page to create data
                } else if p == "0x" {
                    if query.contains_key("data") {
                        let markup_from_admin =
                            crate::controller::templates::admin::admin0x::display_received_data(
                                query
                            );
                        HttpResponse::Ok()
                            .content_type("text/html")
                            .body(markup_from_admin.into_string())
                    } else {
                        let markup =
                            crate::controller::templates::admin::admin0x::render_default_page().await;

                        HttpResponse::Ok().content_type("text/html").body(markup.into_string())
                    }
                    //
                    // admin0x, submit button clicked
                } else {
                    HttpResponse::Ok().content_type("text/html").body("Invalid admin page")
                }
            }
        }
        Err(_) => {
            return HttpResponse::Unauthorized()
                .content_type("text/html; charset=utf-8")
                .body(UNAUTHORIZED_BODY);
        }
    }
}
