use regex::Regex;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::str::FromStr;

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

impl Player {
    pub fn test_data() -> Vec<Self> {
        vec![
            Player {
                id: 1,
                name: "Player1".to_string(),
            },
            Player {
                id: 2,
                name: "Player2".to_string(),
            },
        ]
    }
}

impl Bettor {
    pub fn test_data() -> Vec<Self> {
        vec![
            Bettor {
                uid: 1,
                name: "Bettor1".to_string(),
            },
            Bettor {
                uid: 2,
                name: "Bettor2".to_string(),
            },
        ]
    }
}

// we need this to deserialize the json, even though it seems trivial, it's needed for data validation
#[derive(Deserialize, Debug, Clone)]
pub struct MissingTables {
    pub missing_table: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PlayerBettorRow {
    pub row_entry: i32,
    pub player: Player,
    pub bettor: Bettor,
    pub round: i32,
}

#[derive(Deserialize, Debug)]
pub struct TimesRun {
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub times_run: i32,
}

#[derive(Deserialize, Debug)]
pub struct RowData {
    pub row_entry: i32,
    #[serde(rename = "player.id", deserialize_with = "deserialize_int_or_string")]
    pub player_id: i32,
    #[serde(rename = "bettor.id", deserialize_with = "deserialize_int_or_string")]
    pub bettor_id: i32,
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub round: i32,
}

fn deserialize_int_or_string<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Number(num) => num
            .as_i64()
            .map(|n| n as i32)
            .ok_or_else(|| de::Error::custom("Invalid number for i32")),
        Value::String(s) => i32::from_str(&s).map_err(de::Error::custom),
        _ => Err(de::Error::custom("Expected a string or number")),
    }
}

#[derive(Debug)]
pub struct AlphaNum14(String);

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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum AdminPage {
    Landing,
    TablesAndConstraints,
    ZeroX,
}

impl AdminPage {
    pub fn parse(input: &str) -> Self {
        match input {
            "00" => AdminPage::Landing,
            "01" => AdminPage::TablesAndConstraints,
            "0x" => AdminPage::ZeroX,
            _ => AdminPage::Landing,
        }
    }
}
