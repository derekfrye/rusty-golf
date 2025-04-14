use regex::Regex;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sql_middleware::{
    middleware::{
        ConfigAndPool, CustomDbRow, MiddlewarePool, MiddlewarePoolConnection, QueryAndParams,
    },
    postgres_build_result_set, sqlite_build_result_set, SqlMiddlewareDbError,
};
use std::str::FromStr;

use crate::model::CheckType;

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

impl Default for AlphaNum14 {
    fn default() -> Self {
        AlphaNum14("default".to_string())
    }
}

impl TryFrom<&str> for AlphaNum14 {
    type Error = &'static str;
    
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        AlphaNum14::new(value).ok_or("Invalid alphanumeric string: must be exactly 14 alphanumeric characters")
    }
}

impl std::fmt::Display for AlphaNum14 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AlphaNum14 {
    pub fn new(input: &str) -> Option<Self> {
        // Using a static regex for better performance and safety
        use std::sync::OnceLock;
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let re = REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9]{14}$").expect("Invalid regex pattern - this is a programming error"));
        
        if re.is_match(input) {
            Some(AlphaNum14(input.to_string()))
        } else {
            None
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    // Method removed in favor of Default trait implementation

    pub fn parse(input: &str) -> Result<Self, String> {
        Self::try_from(input).map_err(|_| "Invalid input".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum AdminPage {
    Landing,
    TablesAndConstraints,
    ZeroX,
}

impl AdminPage {
    /// Defaults to the landing page
    pub fn parse(input: &str) -> Self {
        match input {
            "00" => AdminPage::Landing,
            "01" => AdminPage::TablesAndConstraints,
            "0x" => AdminPage::ZeroX,
            _ => AdminPage::Landing,
        }
    }

    pub fn get_page_number(&self) -> &str {
        match self {
            AdminPage::Landing => "00",
            AdminPage::TablesAndConstraints => "01",
            AdminPage::ZeroX => "0x",
        }
    }
}

pub async fn test_is_db_setup(
    config_and_pool: &ConfigAndPool,
    check_type: &CheckType,
) -> Result<Vec<CustomDbRow>, Box<dyn std::error::Error>> {
    let pool = config_and_pool.pool.get().await?;
    let sconn = MiddlewarePool::get_connection(pool).await?;

    let query = match &sconn {
        MiddlewarePoolConnection::Postgres(_xx) => match check_type {
            CheckType::Table => {
                include_str!("sql/schema/postgres/0x_tables_exist.sql")
            }
            _ => {
                return Ok(vec![]);
            }
        },
        MiddlewarePoolConnection::Sqlite(_xx) => match check_type {
            CheckType::Table => {
                include_str!("sql/schema/sqlite/0x_tables_exist.sql")
            }
            _ => {
                return Ok(vec![]);
            }
        },
        &MiddlewarePoolConnection::Mssql(_) => todo!(),
    };

    let query_and_params = QueryAndParams {
        query: query.to_string(),
        params: vec![],
    };

    let res = match sconn {
        MiddlewarePoolConnection::Postgres(mut xx) => {
            let tx = xx.transaction().await?;

            let result_set = {
                let stmt = tx.prepare(&query_and_params.query).await?;
                
                postgres_build_result_set(&stmt, &[], &tx).await?
            };
            tx.commit().await?;
            Ok::<_, SqlMiddlewareDbError>(result_set)
        }
        MiddlewarePoolConnection::Sqlite(xx) => {
            xx.interact(move |xxx| {
                let tx = xxx.transaction()?;
                let result_set = {
                    let mut stmt = tx.prepare(&query_and_params.query)?;
                    
                    sqlite_build_result_set(&mut stmt, &[])?
                };
                tx.commit()?;
                Ok::<_, SqlMiddlewareDbError>(result_set)
            })
            .await?
        }
        MiddlewarePoolConnection::Mssql(_) => todo!()
    }?;

    Ok(res.results)
}

pub async fn create_tables(
    config_and_pool: &ConfigAndPool,
    check_type: &CheckType,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = config_and_pool.pool.get().await?;
    let sconn = MiddlewarePool::get_connection(pool).await?;

    let query = match *check_type {
        CheckType::Table => match &sconn {
            MiddlewarePoolConnection::Postgres(_xx) => match check_type {
                CheckType::Table => [include_str!("sql/schema/postgres/00_event.sql"),
                    include_str!("sql/schema/postgres/02_golfer.sql"),
                    include_str!("sql/schema/postgres/03_bettor.sql"),
                    include_str!("sql/schema/postgres/04_event_user_player.sql"),
                    include_str!("sql/schema/postgres/05_eup_statistic.sql")]
                .join("\n"),
                _ => {
                    return Ok(());
                }
            },
            MiddlewarePoolConnection::Sqlite(_xx) => match check_type {
                CheckType::Table => [include_str!("sql/schema/sqlite/00_event.sql"),
                    include_str!("sql/schema/sqlite/02_golfer.sql"),
                    include_str!("sql/schema/sqlite/03_bettor.sql"),
                    include_str!("sql/schema/sqlite/04_event_user_player.sql"),
                    include_str!("sql/schema/sqlite/05_eup_statistic.sql")]
                .join("\n"),
                _ => {
                    return Ok(());
                }
            },
            &MiddlewarePoolConnection::Mssql(_) => todo!(),
        },
        CheckType::Constraint => {
            return Ok(());
        }
    };

    let query_and_params = QueryAndParams {
        query: query.to_string(),
        params: vec![],
    };

    match sconn {
        MiddlewarePoolConnection::Postgres(mut xx) => {
            let tx = xx.transaction().await?;

            tx.batch_execute(&query_and_params.query).await?;
            tx.commit().await?;
            Ok::<_, SqlMiddlewareDbError>(())
        }
        MiddlewarePoolConnection::Sqlite(xx) => {
            xx.interact(move |xxx| {
                let tx = xxx.transaction()?;
                tx.execute_batch(&query_and_params.query)?;

                tx.commit()?;
                Ok::<_, SqlMiddlewareDbError>(())
            })
            .await?
        }
        MiddlewarePoolConnection::Mssql(_) => todo!()
    }?;

    Ok(())
}
