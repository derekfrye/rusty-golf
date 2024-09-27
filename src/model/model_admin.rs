use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Bettor {
    pub uid: i32,
    pub name: String,
}

#[derive(Debug)]
pub struct AlphaNum14(String);

#[derive(Debug)]
#[allow(dead_code)]
pub struct PlayerBettorRow {
    pub row_entry: i32,
    pub player: Player,
    pub bettor: Bettor,
    pub round: i32,
}

#[derive(Deserialize, Debug)]
pub struct RowData {
    pub row_entry: i32,
    #[serde(rename = "player.id")]
    pub player_id: i32,
    #[serde(rename = "bettor.id")]
    pub bettor_id: i32,
    pub round: i32,
}

impl AlphaNum14 {
    pub fn new(input: &str) -> Option<Self> {
        let re = Regex::new(r"^[a-zA-Z0-9]{14}$").unwrap();
        if re.is_match(input) {
            Some(AlphaNum14(input.to_string()))
        } else {
            None
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn default() -> Self {
        AlphaNum14("default".to_string())
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        if let Some(alpha_num) = AlphaNum14::new(input) {
            Ok(alpha_num)
        } else {
            Err("Invalid input".to_string())
        }
    }
}
