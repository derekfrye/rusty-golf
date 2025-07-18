use crate::model::AllBettorScoresByRound;
use maud::{Markup, html};

pub fn render_summary_scores(grouped_data: &AllBettorScoresByRound) -> Markup {
    html! {
        h3 { "Summary Scores by Round" }
        table class="styled-table" {
            thead {
                tr {
                    th { "Player" }
                    th { "Round 1" }
                    th { "Round 2" }
                    th { "Round 3" }
                    th { "Round 4" }
                    th { "Total" }
                }
            }
            tbody {
                @for summary_score in &grouped_data.summary_scores {
                    tr {
                        td { (summary_score.bettor_name) }
                        @for (idx, score) in summary_score.scores_aggregated_by_golf_grp_by_rd.iter().enumerate() {
                            @let round_num = idx + 1;
                            @if round_num <= 4 {
                                td { (score) }
                            }
                        }
                        @let total = summary_score.scores_aggregated_by_golf_grp_by_rd.iter().sum::<isize>();
                        td { (total) }
                    }
                }
            }
        }
    }
}
