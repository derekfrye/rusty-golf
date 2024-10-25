use crate::model::{
    self,
    admin_model::{AlphaNum14, Bettor, Player, PlayerBettorRow, RowData},
};

use actix_web::{web, HttpResponse};
// use actix_web::{web, HttpResponse};
use maud::{html, Markup};
use serde_json::json;
use std::{collections::HashMap, env};
// use serde_json::json;

// Render the main page
pub async fn render_default_page(players: &Vec<Player>, bettors: &Vec<Bettor>) -> Markup {
    let admin_00 = crate::controller::templates::admin::admin00::render_default_page().await;
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Table of Entries" }
                // Include htmx
                script src="https://unpkg.com/htmx.org@1.9.12" {}
                link rel="stylesheet" type="text/css" href="static/styles.css";
            }
            body {
                table class="hidden" {
                    thead {
                        tr {
                            th { "Player" }
                            th { "Bettor" }
                            th { "Round" }
                        }
                    }
                    tbody id="table-body" {
                        tr {
                            td {
                                select class="player-select" {
                                    @for player in players {
                                        option value=(player.id) { (player.name) }
                                    }
                                }
                            }
                            td {
                                select class="bettor-select" {
                                    @for bettor in bettors {
                                        option value=(bettor.uid) { (bettor.name) }
                                    }
                                }
                            }
                            td {
                                select class="round-select" {
                                    option value="1" { "1" }
                                    option value="2" { "2" }
                                    option value="3" { "3" }
                                    option value="4" { "4" }
                                }
                            }
                        }
                    }
                }
                button type="button" id="add-row" class="hidden" { "Add Row" }
                button type="button" id="submit" class="hidden" { "Submit" }
                div id="results" {}
                div id="admin-00" {
                    (admin_00)
                }
                script src="static/admin.js" {}
            }
        }
    }
}

pub fn display_received_data(players: Vec<Player>, bettors: Vec<Bettor>, data: String) -> Markup {
    // Deserialize the data
    let data: Vec<RowData> = match serde_json::from_str(&data) {
        Ok(d) => d,
        Err(e) => {
            return html! {
                p { "Invalid data: " (e) }
            };
        }
    };

    // Create HashMaps for quick ID lookup
    let player_map: HashMap<i32, &Player> = players.iter().map(|p| (p.id, p)).collect();
    let bettor_map: HashMap<i32, &Bettor> = bettors.iter().map(|b| (b.uid, b)).collect();

    let mut player_bettor_rows = Vec::new();

    for row_data in data {
        // Get the Player by ID
        let player = match player_map.get(&row_data.player_id) {
            Some(p) => *p,
            None => {
                return html! {
                    p { "Invalid player ID: " (row_data.player_id) }
                };
            }
        };

        // Get the Bettor by ID
        let bettor = match bettor_map.get(&row_data.bettor_id) {
            Some(b) => (*b).clone(),
            None => {
                return html! {
                    p { "Invalid bettor ID: " (row_data.bettor_id) }
                };
            }
        };

        let player_bettor_row = PlayerBettorRow {
            row_entry: row_data.row_entry,
            player: player.clone(),
            bettor: bettor.clone(),
            round: row_data.round,
        };

        player_bettor_rows.push(player_bettor_row);
    }

    // For demonstration, display the received data
    html! {
        // (render_page(&players, &bettors))
        div {
            h2 { "Received Data" }
            table {
                thead {
                    tr {
                        th { "Player" }
                        th { "Bettor" }
                        th { "Round" }
                    }
                }
                tbody {
                    @for row in player_bettor_rows {
                        tr {
                            td { (row.player.name) }
                            td { (row.bettor.name) }
                            td { (row.round) }
                        }
                    }
                }
            }
        }
    }
}

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

    let data = query
        .get("data")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let missing_tables = query
        .get("admin00_missing_tables")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let times_run = query
        .get("times_run")
        .unwrap_or(&String::new())
        .trim()
        .to_string();

    let player_vec = vec![
        model::admin_model::Player {
            id: 1,
            name: "Player1".to_string(),
        },
        model::admin_model::Player {
            id: 2,
            name: "Player2".to_string(),
        },
    ];
    let bettor_vec = vec![
        model::admin_model::Bettor {
            uid: 1,
            name: "Bettor1".to_string(),
        },
        model::admin_model::Bettor {
            uid: 2,
            name: "Bettor2".to_string(),
        },
    ];

    let body = r#"
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

    // check query string token against the token this container was started with
    match env::var("TOKEN") {
        Ok(env_token) => {
            if env_token != token.value() {
                return HttpResponse::Unauthorized()
                    .content_type("text/html; charset=utf-8")
                    .body(body);
            } else {
                // authorized
                if !missing_tables.is_empty() {
                    let markup_from_admin =
                        crate::controller::templates::admin::admin00::create_tables(
                            missing_tables,
                            times_run,
                        )
                        .await;

                    // dbg!("markup_from_admin", &markup_from_admin.times_run_int);

                    let header = json!({
                        "reenablebutton": "1",
                        "times_run": markup_from_admin.times_run_int
                    });

                    HttpResponse::Ok()
                        .content_type("text/html")
                        .insert_header(("HX-Trigger", header.to_string())) // Add the HX-Trigger header, this tells the create table button to reenable (based on a fn in admin.js)
                        .body(markup_from_admin.html.into_string())
                } else if data.is_empty() {
                    let markup = crate::controller::templates::admin::admin::render_default_page(
                        &player_vec,
                        &bettor_vec,
                    )
                    .await;

                    HttpResponse::Ok()
                        .content_type("text/html")
                        .body(markup.into_string())
                } else {
                    let markup_from_admin =
                        crate::controller::templates::admin::admin::display_received_data(
                            player_vec, bettor_vec, data,
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
                .body(body);
        }
    }
}
