use rusty_golf_actix::model::{
    AllBettorScoresByRound, BettorScoreByRound, DetailedScore, SummaryDetailedScores,
};

pub(super) fn build_detailed_scores() -> SummaryDetailedScores {
    let data = [
        ("Player1", "Scottie Scheffler", 1_234_567, [-3, 0]),
        ("Player1", "Collin Morikawa", 1_234_568, [-2, 0]),
        ("Player1", "Min Woo Lee", 1_234_569, [0, 0]),
        ("Player2", "Bryson DeChambeau", 1_234_570, [-1, 0]),
        ("Player2", "Justin Thomas", 1_234_571, [1, 0]),
        ("Player2", "Hideki Matsuyama", 1_234_572, [0, 0]),
        ("Player3", "Rory McIlroy", 1_234_573, [0, 0]),
        ("Player3", "Ludvig Åberg", 1_234_574, [1, 0]),
        ("Player3", "Sepp Straka", 1_234_575, [0, 0]),
        ("Player4", "Brooks Koepka", 1_234_576, [0, 0]),
        ("Player4", "Viktor Hovland", 1_234_577, [0, 0]),
        ("Player4", "Jason Day", 1_234_578, [0, 0]),
        ("Player5", "Xander Schauffele", 1_234_579, [3, 0]),
        ("Player5", "Jon Rahm", 1_234_580, [1, 0]),
        ("Player5", "Will Zalatoris", 1_234_581, [0, 0]),
    ];
    SummaryDetailedScores {
        detailed_scores: data
            .into_iter()
            .map(
                |(bettor_name, golfer_name, golfer_espn_id, scores)| DetailedScore {
                    bettor_name: bettor_name.to_string(),
                    golfer_name: golfer_name.to_string(),
                    golfer_espn_id,
                    rounds: vec![0, 1],
                    scores: vec![scores[0], scores[1]],
                },
            )
            .collect(),
    }
}

pub(super) fn build_summary_scores() -> AllBettorScoresByRound {
    let scores = [
        ("Player1", [-5, 0]),
        ("Player2", [0, 0]),
        ("Player3", [1, 0]),
        ("Player4", [0, 0]),
        ("Player5", [4, 0]),
    ];
    AllBettorScoresByRound {
        summary_scores: scores
            .into_iter()
            .map(|(bettor_name, scores)| BettorScoreByRound {
                bettor_name: bettor_name.to_string(),
                computed_rounds: vec![0, 1],
                scores_aggregated_by_golf_grp_by_rd: scores.to_vec(),
            })
            .collect(),
    }
}
