use maud::{html, Markup};
// use serde_json::json;
use regex::Regex;

// Define the Player and Bettor structs
pub struct Player {
    pub id: i32,
    pub name: String,
}

pub struct Bettor {
    pub uid: i32,
    pub name: String,
}

#[derive(Debug)]
pub struct AlphaNum14(String);

impl AlphaNum14 {
    pub fn new(input: &str) -> Option<Self> {
        let re = Regex::new(r"^[a-zA-Z0-9]{14}$").unwrap();
        if re.is_match(input) {
            Some(AlphaNum14(input.to_string()))
        } else {
            None
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn default() -> Self {
        AlphaNum14("default".to_string())
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        if let Some(alpha_num) = AlphaNum14::new(input) {
            Ok(alpha_num)
        } else {
            Err("Invalid input".to_string())
        }
    }
}

// Render the main page
pub fn render_page(players: &Vec<Player>, bettors: &Vec<Bettor>) -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Table of Entries" }
                // Include htmx
                script src="https://unpkg.com/htmx.org@1.9.12" {}
            }
            body {
                table {
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
                button type="button" id="add-row" { "Add Row" }
                button type="button" id="submit" { "Submit" }
                div id="results" {}
                script src="static/admin.js" {}
                // Optional CSS for animation
                style {
                    r#"
                    .animate {
                        animation: fadeIn 0.5s;
                    }
                    @keyframes fadeIn {
                        from { opacity: 0; }
                        to { opacity: 1; }
                    }
                    "#
                }
            }
        }
    }
}
