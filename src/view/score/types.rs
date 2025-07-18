use crate::model::{LineScore, StringStat};

#[derive(Debug, Clone)]
pub struct Bar {
    pub score: i32,
    pub direction: Direction,
    pub start_position: f32,
    pub width: f32,
    pub round: i32,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct GolferBars {
    pub short_name: String,
    pub total_score: isize,
    pub bars: Vec<Bar>,
    pub is_even: bool,
}

#[derive(Debug, Clone)]
pub struct BettorData {
    pub bettor_name: String,
    pub golfers: Vec<GolferData>,
}

#[derive(Debug, Clone)]
pub struct RefreshData {
    pub last_refresh: String,
    pub last_refresh_source: crate::model::RefreshSource,
}

#[derive(Debug, Clone)]
pub struct GolferData {
    pub golfer_name: String,
    pub linescores: Vec<LineScore>,
    pub tee_times: Vec<StringStat>,
}