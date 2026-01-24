use crate::model::ScoreData;
use maud::{Markup, html};

#[must_use]
pub fn render_scoreboard(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty(){

            @let grouped_bettors = &data.bettor_struct;

            h2 { "Scoreboard" }

            table class="styled-table" {
                thead {
                    tr {
                        th { "PLACE" }
                        th { "PLAYER" }
                        th { "SCORE" }
                    }
                }
                tbody {
                    @for (idx, bettor) in grouped_bettors.iter().enumerate() {
                        @let emoji = rank_emoji(idx);
                        tr {
                            td {
                                (bettor.scoreboard_position_name)
                                @if let Some(mark) = emoji {
                                    " " (mark)
                                }
                            }
                            td { (bettor.bettor_name) }
                            td { (bettor.total_score) }
                        }
                    }
                }
            }
        }
        @else {
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
                        th { "Tee" }
                        th { "Holes" }
                        th { "Score" }
                        th { "Tee" }
                        th { "Holes" }
                        th { "Score" }
                        th { "Tee" }
                        th { "Holes" }
                        th { "Score" }
                        th { "Tee" }
                        th { "Holes" }
                        th { "Score" }
                    }
                }
                tbody {
                    tr {
                        td { "Player" }
                        td { "Pick" }
                        td colspan="12" { "No data available" }
                    }
                }
            }
        }
    }
}

fn rank_emoji(index: usize) -> Option<&'static str> {
    match index {
        0 => Some("🥇"),
        1 => Some("🥈"),
        2 => Some("🥉"),
        3 => Some("😐"),
        4 => Some("🪨"),
        _ => None,
    }
}
