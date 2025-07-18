use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Statistic {
    pub eup_id: i64,
    pub rounds: Vec<IntStat>,
    pub round_scores: Vec<IntStat>,
    pub tee_times: Vec<StringStat>,
    pub holes_completed_by_round: Vec<IntStat>,
    pub line_scores: Vec<LineScore>,
    pub total_score: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StringStat {
    pub val: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IntStat {
    pub val: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LineScore {
    pub round: i32,
    pub hole: i32,
    pub score: i32,
    pub par: i32,
    pub score_display: ScoreDisplay,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum ScoreDisplay {
    DoubleCondor,
    Condor,
    Albatross,
    Eagle,
    Birdie,
    Par,
    Bogey,
    DoubleBogey,
    TripleBogey,
    QuadrupleBogey,
    QuintupleBogey,
    SextupleBogey,
    SeptupleBogey,
    OctupleBogey,
    NonupleBogey,
    DodecupleBogey,
}

impl ScoreDisplay {
    pub fn from_i32(i: i32) -> Self {
        match i {
            -5 => ScoreDisplay::DoubleCondor,
            -4 => ScoreDisplay::Condor,
            -3 => ScoreDisplay::Albatross,
            -2 => ScoreDisplay::Eagle,
            -1 => ScoreDisplay::Birdie,
            0 => ScoreDisplay::Par,
            1 => ScoreDisplay::Bogey,
            2 => ScoreDisplay::DoubleBogey,
            3 => ScoreDisplay::TripleBogey,
            4 => ScoreDisplay::QuadrupleBogey,
            5 => ScoreDisplay::QuintupleBogey,
            6 => ScoreDisplay::SextupleBogey,
            7 => ScoreDisplay::SeptupleBogey,
            8 => ScoreDisplay::OctupleBogey,
            9 => ScoreDisplay::NonupleBogey,
            10 => ScoreDisplay::DodecupleBogey,
            _ => ScoreDisplay::Par,
        }
    }
}

impl From<i32> for ScoreDisplay {
    fn from(value: i32) -> Self {
        Self::from_i32(value)
    }
}