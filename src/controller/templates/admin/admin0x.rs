use std::collections::HashMap;

use crate::model::admin_model::{Bettor, Player, PlayerBettorRow, RowData};

use maud::{html, Markup};

// Render the main page
pub async fn render_default_page(players: &Vec<Player>, bettors: &Vec<Bettor>) -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Golf Admin 0x" }
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
