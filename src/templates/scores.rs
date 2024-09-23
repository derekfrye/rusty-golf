use crate::model::ScoreData;

// use actix_web::{web, HttpResponse, Responder};
use maud::{html, Markup};

pub fn render_scores_template(total_cache: &ScoreData) -> Markup {
    html! {
        @if !total_cache.last_refresh.is_empty() {
            p class="refresh" {
                "Last refreshed from ESPN " (total_cache.last_refresh) " ago."
            }
        }

        h3 { "Scoreboard" }

        table class="styled-table" {
            thead {
                tr {
                    th { "PLACE" }
                    th { "PLAYER" }
                    th { "SCORE" }
                }
            }
            tbody {
                @for bettor in &total_cache.bettor_struct {
                    tr {
                        td { (bettor.scoreboard_position_name) }
                        td { (bettor.bettor_name) }
                        td { (bettor.total_score) }
                    }
                }
            }
        }

        @if total_cache.score_struct.is_empty() {
            table id="scores-table-1" {
                thead {
                    tr {
                        th rowspan="2" { "Player" }
                        th rowspan="2" { "Pick" }
                        th colspan="3" { "Round 1" }
                        th colspan="3" { "Round 2" }
                        th colspan="3" { "Round 3" }
                        th colspan="3" { "Round 4" }
                        th rowspan="2" { "Total" }
                    }
                    tr {
                        th { "Tee Time" }
                        th { "Score" }
                        th { "Tee Time" }
                        th { "Score" }
                        th { "Tee Time" }
                        th { "Score" }
                        th { "Tee Time" }
                        th { "Score" }
                    }
                }
                tbody {
                    tr {
                        td colspan="15" { "No scores available." }
                    }
                }
            }
        }
    }
}

// async fn scores(data: web::Data<ScoreData>) -> impl Responder {
//     let markup = render_scores_template(&data);
//     HttpResponse::Ok()
//         .content_type("text/html")
//         .body(markup.into_string())
// }
