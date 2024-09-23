use std::collections::HashMap;

use crate::model::{ScoreData, Scores, SummaryScore, SummaryScores};

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

                                @let total = 0;
                                @for (idx, _round) in summary_score.computed_rounds.iter().enumerate() {
                                    @let score = summary_score.new_scores[idx];
                                    td { (score) }
                                    // @let total = total + score;
                                }
                                td { (total) }
                            }
                        }
                    }
                }
            }

            h3 { "Details" }

@let grouped_scores = group_by_scores(data.score_struct.clone());
@for (group, scores) in &grouped_scores {
    table id=(format!("scores-table-{}", group)) {
        thead {
            tr {
                th class="topheader" rowspan="2" { "Player" }
                th class="topheader" rowspan="2" { "Pick" }

                // @let mut max_rounds = 0;
                // @for score in scores {
                //     @let rounds = score.detailed_statistics.tee_times.len();
                //     @if rounds > max_rounds {
                //      max_rounds = rounds;
                //     }
                // }

                @for round in 0..4 {
                // @for round in 0..max_rounds {
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
                // @let mut itr = 0;
                @let z_vec = vec!["Tee Time (CT)", "Holes Compl.", "Score", "Total"];
                @for round in 0..4 {
                //@for round in 0..max_rounds {
                @for a in 1..4 {
                    th class="sortable hideable" data-round=({ round + 1 }) onclick=(format!("sortTable('scores-table-{}', {})", group, a)) {
                        (z_vec[a])
                    }
                    // @{ itr += 1; }
                    // th class="sortable hideable" data-round=({ round + 1 }) onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                    //     "Tee Time (CT)"
                    // }
                    // // @{ itr += 1; }
                    // th class="sortable hideable" data-round=({ round + 1 }) onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                    //     "Holes Compl."
                    // }
                    // // @{ itr += 1; }
                    // th class="sortable" onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                    //     "Score"
                    // }
                }
            }
                // @{ itr += 1; }
                // th class="sortable" onclick=(format!("sortTable('scores-table-{}', {})", group, itr)) {
                //     "Total"
                // }
            }
        }
        tbody {
            @for score in scores {
                tr {
                    td { (score.bettor_name) }
                    td { (score.golfer_name) }
                    @let stats = &score.detailed_statistics;
                    @for index in 0..4 {
                    // @for index in 0..max_rounds {
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

fn group_by_scores(scores: Vec<Scores>) -> HashMap<i32, Vec<Scores>> {
    let mut grouped_scores: HashMap<i32, Vec<Scores>> = HashMap::new();

    for score in scores {
        grouped_scores.entry(score.group.try_into().unwrap())
            .or_insert_with(Vec::new)
            .push(score);
    }

    grouped_scores
}

fn group_by_bettor_name_and_round(scores: &Vec<Scores>) -> SummaryScores {
    let mut bettor_round_scores: HashMap<String, HashMap<usize, i64>> = HashMap::new();

    // Accumulate scores by bettor and round
    for score in scores {
        let bettor_name = &score.bettor_name;
        for (round_idx, round_score) in score.detailed_statistics.scores.iter().enumerate() {
            let rounds = bettor_round_scores.entry(bettor_name.clone()).or_insert_with(HashMap::new);
            *rounds.entry(round_idx).or_insert(0) += round_score.val as i64;
        }
    }

    let mut summary_scores = SummaryScores {
        summary_scores: Vec::new(),
    };
    let mut bettor_names: Vec<String> = Vec::new();

    // Preserve order of bettor names
    for score in scores {
        let bettor_name = &score.bettor_name;
        if bettor_round_scores.contains_key(bettor_name) && !bettor_names.contains(bettor_name) {
            bettor_names.push(bettor_name.clone());
        }
    }

    // Preserve order of rounds
    for bettor_name in &bettor_names {
        if let Some(rounds) = bettor_round_scores.get(bettor_name) {
            let mut computed_rounds: Vec<i64> = Vec::with_capacity(rounds.len());
            let mut new_scores: Vec<i64> = Vec::with_capacity(rounds.len());

            for (round_idx, &score) in rounds.iter() {
                computed_rounds.push(*round_idx as i64);
                new_scores.push(score.try_into().unwrap());
            }

            summary_scores.summary_scores.push(SummaryScore {
                bettor_name: bettor_name.clone(),
                computed_rounds ,
                new_scores,
            });
        }
    }

    summary_scores
}