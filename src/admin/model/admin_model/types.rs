use serde::Deserialize;
use serde_json::Value;
use std::str::FromStr;
use serde::de;
use serde::Deserializer;

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

#[derive(Deserialize, Debug, Clone)]
pub struct MissingDbObjects {
    pub missing_object: String,
}

pub fn deserialize_int_or_string<'de, D>(deserializer: D) -> Result<i32, D::Error>
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