use crate::model::ScoreData;

use maud::{html, Markup};

pub fn render_scores_template(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty() {
            p class="refresh" {
                "Last refreshed from ESPN " (data.last_refresh) " ago."
            }

            @let grouped_bettors = &data.bettor_struct;

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
                    @for bettor in grouped_bettors {
                        tr {
                            td { (bettor.scoreboard_position_name) }
                            td { (bettor.bettor_name) }
                            td { (bettor.total_score) }
                        }
                    }
                }
            }

            @let summary_scores = group_by_bettor_name_and_round(&data.score_struct);

            @if !summary_scores.summary_scores.is_empty() {
                h3 { "Summary" }

                table class="summary" {
                    thead {
                        tr {
                            th { "Player" }
                            @let num_rounds = summary_scores.summary_scores[0].computed_rounds.len();
                            @for round in 0..num_rounds {
                                th { "R" (round + 1) }
                            }
                            th { "Tot" }
                        }
                    }
                    tbody {
                        @for summary_score in &summary_scores.summary_scores {
                            tr {
                                td { (summary_score.bettor_name) }

                                @let mut total = 0;
                                @for (idx, _round) in summary_score.computed_rounds.iter().enumerate() {
                                    @let score = summary_score.new_scores[idx];
                                    td { (score) }
                                    @let total = total + score;
                                }
                                td { (total) }
                            }
                        }
                    }
                }
            }

            h3 { "Details" }

@let grouped_scores = group_by_scores(&data.score_struct);
@for (group, scores) in &grouped_scores {
    table id=(format!("scores-table-{}", group)) {
        thead {
            tr {
                th class="topheader" rowspan="2" { "Player" }
                th class="topheader" rowspan="2" { "Pick" }

                @let mut max_rounds = 0;
                @for score in scores {
                    @let rounds = score.detailed_statistics.tee_times.len();
                    @if rounds > max_rounds {
                         max_rounds = rounds; 
                    }
                }

                @for round in 0..max_rounds {
                    th class="topheader shrinkable" colspan="3" data-round=({ round + 1 }) {
                        span class="toggle" data-round=({ round + 1 }) onclick=(format!("toggleRound({})", round + 1)) {
                            "Round " (round + 1)
                        }
                        br;
                        span class="kindatiny" data-round=({ round + 1 }) onclick=(format!("toggleRound({})", round + 1)) {
                            "Tap to shrink"
                        }
                    }
                }
                th class="topheader" { i { "Totals" } }
            }
            tr {
                @let mut itr = 0;
                @for round in 0..max_rounds {
                    @{ itr += 1; }
                    th class="sortable hideable" data-round=({ round + 1 }) onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                        "Tee Time (CT)"
                    }
                    @{ itr += 1; }
                    th class="sortable hideable" data-round=({ round + 1 }) onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                        "Holes Compl."
                    }
                    @{ itr += 1; }
                    th class="sortable" onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                        "Score"
                    }
                }
                @{ itr += 1; }
                th class="sortable" onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                    "Total"
                }
            }
        }
        tbody {
            @for score in scores {
                tr {
                    td { (score.bettor_name) }
                    td { (score.golfer_name) }
                    @let stats = &score.detailed_statistics;
                    @for index in 0..max_rounds {
                        @if index < stats.tee_times.len() {
                            td class="cells hideable teetime" data-round=({ index + 1 }) {
                                (stats.tee_times[index].val)
                            }
                            td class="cells hideable" data-round=({ index + 1 }) {
                                @if index < stats.holes_completed.len() {
                                    (stats.holes_completed[index].val)
                                } @else {
                                    "N/A"
                                }
                            }
                            td class="cells" data-round=({ index + 1 }) {
                                @if index < stats.scores.len() {
                                    (stats.scores[index].val)
                                } @else {
                                    "N/A"
                                }
                            }
                        } @else {
                            td class="cells hideable" data-round=({ index + 1 }) { "N/A" }
                            td class="cells hideable" data-round=({ index + 1 }) { "N/A" }
                            td class="cells" data-round=({ index + 1 }) { "N/A" }
                        }
                    }
                    td { (stats.total_score) }
                }
            }
        }
    }
}

        } @else {
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
