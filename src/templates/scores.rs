use std::collections::{BTreeMap, HashMap};

use crate::model::{ScoreData, Scores, SummaryScore, SummaryScores};

use maud::{html, Markup};

pub fn render_scores_template(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty() {
            (render_scoreboard(data))
            (render_summary_scores(data))
            (render_score_detail(data))
        }
    }
}

fn render_scoreboard(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty(){
            h3 { "Scoreboard" }
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

fn render_summary_scores(data: &ScoreData) -> Markup {
    html! {
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
                            @for (idx, _round) in summary_score.computed_rounds.iter().enumerate() {
                                @let score = summary_score.new_scores[idx];
                                td { (score) }
                            }
                            @let total = summary_score.new_scores.iter().sum::<isize>();
                            td { (total) }
                        }
                    }
                }
            }
        }
    }
}

fn render_thead(max_len_of_tee_times_in_rounds: usize, group: &usize) -> Markup {
    html! {
        thead {
            tr {
                th class="topheader" rowspan="2" {
                    "Player"
                }
                th class="topheader" rowspan="2" {
                    "Pick"
                }

                @for round in 0..max_len_of_tee_times_in_rounds {
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
                th class="topheader" {
                    i {
                        "Totals"
                    }
                }
            }
            tr {
                @let z_vec = vec!["Tee Time (CT)", "Holes Compl.", "Score", ];
                @let z_len = z_vec.len();
                @for round in 0..max_len_of_tee_times_in_rounds {
                    // for each of the 4 columns, add tee time, holes completed, and score
                    @for a in 0..z_len {
                        @let c = round * z_vec.len() + a +1;
                        th class="sortable hideable" data-round=({
                            round + 1
                        })
                        onclick=(format!("sortTable('scores-table-{}', {})", group, c)) {
                            (z_vec[a])
                        }
                    }
                }
                @let d = max_len_of_tee_times_in_rounds * z_vec.len() +1;
                th class="sortable" onclick=(format!("sortTable('scores-table-{}', {})", group, d)) {
                    "Total"
                }
            }
        }
    }
}

fn render_score_detail(data: &ScoreData) -> Markup {
    html! {
        h3 { "Details" }
         @let grouped_scores = group_by_scores(data.score_struct.clone());
        @for (group, scores) in &grouped_scores {
            @let max_len_of_tee_times = scores.iter().map(|score| score.detailed_statistics.tee_times.len()).max().unwrap_or(0);
            table id=(format!("scores-table-{}", group)) {
                (render_thead(max_len_of_tee_times, group) )
                tbody {
                    @for score in scores {
                        tr {
                            td { (score.bettor_name) }
                            td { (score.golfer_name) }

                            @let stats = &score.detailed_statistics;
                            @for index in 0..max_len_of_tee_times {
                                @let tee_time_len = stats.tee_times.len();
                                @if index < tee_time_len {
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
    }
}

fn group_by_scores(scores: Vec<Scores>) -> Vec<(usize, Vec<Scores>)> {
    let mut grouped_scores: HashMap<usize, Vec<Scores>> = HashMap::new();

    for score in scores {
        grouped_scores
            .entry(score.group as usize)
            .or_insert_with(Vec::new)
            .push(score);
    }

    let x = sort_scores(grouped_scores);

    x
}

fn sort_scores(grouped_scores: HashMap<usize, Vec<Scores>>) -> Vec<(usize, Vec<Scores>)> {
    let mut sorted_scores: Vec<(usize, Vec<Scores>)> = grouped_scores.into_iter().collect();

    sorted_scores.sort_by_key(|(group, _)| *group); // Sort by the `group` key

    sorted_scores
}

fn group_by_bettor_name_and_round(scores: &Vec<Scores>) -> SummaryScores {
    // key = bettor, value = hashmap of rounds and the corresponding score
    let mut rounds_by_bettor_storing_score_val: HashMap<String, Vec<(isize, isize)>> =
        HashMap::new();

    // Accumulate scores by bettor and round
    for score in scores {
        let bettor_name = &score.bettor_name;

        // for debug watching
        // let golfers_name = &score.golfer_name;
        // let _ = golfers_name.len();

        for (round_idx, round_score) in score.detailed_statistics.scores.iter().enumerate() {
            let a_single_bettors_scores = rounds_by_bettor_storing_score_val
                .entry(bettor_name.clone())
                .or_insert_with(Vec::new);
            a_single_bettors_scores.push((round_idx.try_into().unwrap(), round_score.val as isize));

            // for debug watching
            // let golfers_namex = &score.golfer_name;
            // let _ = golfers_namex.len();
        }
    }

    let mut summary_scores = SummaryScores {
        summary_scores: Vec::new(),
    };
    let mut bettor_names: Vec<String> = Vec::new();

    // Preserves order of bettors
    for score in scores {
        let bettor_name = &score.bettor_name;
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name)
            && !bettor_names.contains(bettor_name)
        {
            bettor_names.push(bettor_name.clone());
        }
    }

    // Preserves order of bettors
    // this actually just needs to sum all the scores where the rounds are 0, store that val, sum all scores where rounds are 1, store that value, etc
    for bettor_name in &bettor_names {
        if let Some(_) = rounds_by_bettor_storing_score_val.get(bettor_name) {
            let res1 = rounds_by_bettor_storing_score_val
                .get(bettor_name)
                .unwrap()
                .iter();

            let result = res1
                .fold(BTreeMap::new(), |mut acc, &(k, v)| {
                    *acc.entry(k).or_insert(0) += v;
                    acc
                })
                .into_iter()
                .collect::<Vec<(isize, isize)>>();

            let (computed_rounds, new_scores): (Vec<isize>, Vec<isize>) =
                result.iter().cloned().unzip();

            summary_scores.summary_scores.push(SummaryScore {
                bettor_name: bettor_name.clone(),
                computed_rounds,
                new_scores,
            });
        }
    }

    summary_scores
}
