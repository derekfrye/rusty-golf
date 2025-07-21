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
    #[must_use]
    pub fn from_i32(i: i32) -> Self {
        match i {
            -5 => Self::DoubleCondor,
            -4 => Self::Condor,
            -3 => Self::Albatross,
            -2 => Self::Eagle,
            -1 => Self::Birdie,
            1 => Self::Bogey,
            2 => Self::DoubleBogey,
            3 => Self::TripleBogey,
            4 => Self::QuadrupleBogey,
            5 => Self::QuintupleBogey,
            6 => Self::SextupleBogey,
            7 => Self::SeptupleBogey,
            8 => Self::OctupleBogey,
            9 => Self::NonupleBogey,
            10 => Self::DodecupleBogey,
            _ => Self::Par,
        }
    }
}

impl From<i32> for ScoreDisplay {
    fn from(value: i32) -> Self {
        Self::from_i32(value)
    }
}
