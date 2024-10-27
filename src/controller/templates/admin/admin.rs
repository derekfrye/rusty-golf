use crate::model::{ self, admin_model::AlphaNum14 };

use actix_web::{ web, HttpResponse };
// use actix_web::{web, HttpResponse};
// use maud::{ html, Markup };
use serde_json::json;
use std::{ collections::HashMap, env, };
// use serde_json::json;

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
    let data = query.get("data").unwrap_or(&String::new()).trim().to_string();
    let missing_tables = query
        .get("admin00_missing_tables")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let times_run = query.get("times_run").unwrap_or(&String::new()).trim().to_string();

    let player_vec = vec![
        model::admin_model::Player {
            id: 1,
            name: "Player1".to_string(),
        },
        model::admin_model::Player {
            id: 2,
            name: "Player2".to_string(),
        }
    ];
    let bettor_vec = vec![
        model::admin_model::Bettor {
            uid: 1,
            name: "Bettor1".to_string(),
        },
        model::admin_model::Bettor {
            uid: 2,
            name: "Bettor2".to_string(),
        }
    ];

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
                if  p=="00" {
                    if !missing_tables.is_empty() {
                      get_html_for_creat_tables(missing_tables,times_run).await
                    }else{
                    let x=crate::controller::templates::admin::admin00::render_default_page().await;
                    HttpResponse::Ok().content_type("text/html").body(x.into_string())
                    }
                //
                // admin0x, render the page to create data
                } else if data.is_empty() {
                    let markup = crate::controller::templates::admin::admin0x::render_default_page(
                        &player_vec,
                        &bettor_vec
                    ).await;

                    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
                //
                // admin0x, submit button clicked
                } else {
                    let markup_from_admin =
                        crate::controller::templates::admin::admin0x::display_received_data(
                            player_vec,
                            bettor_vec,
                            data
                        );
                    HttpResponse::Ok()
                        .content_type("text/html")
                        .body(markup_from_admin.into_string())
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


async fn get_html_for_creat_tables(missing_tables:String,times_run: String) -> HttpResponse {
    let markup_from_admin =
                        crate::controller::templates::admin::admin00::create_tables(
                            missing_tables,
                            times_run
                        ).await;

                    // dbg!("markup_from_admin", &markup_from_admin.times_run_int);

                    let header =
                        json!({
                        "reenablebutton": "1",
                        "times_run": markup_from_admin.times_run_int
                    });

                    HttpResponse::Ok()
                        .content_type("text/html")
                        .insert_header(("HX-Trigger", header.to_string())) // Add the HX-Trigger header, this tells the create table button to reenable (based on a fn in admin.js)
                        .body(markup_from_admin.html.into_string())
}