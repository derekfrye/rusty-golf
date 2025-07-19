use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum AdminPage {
    Landing,
    TablesAndConstraints,
    ZeroX,
}

impl AdminPage {
    /// Defaults to the landing page
    #[must_use]
    pub fn parse(input: &str) -> Self {
        match input {
            "00" => Self::Landing,
            "01" => Self::TablesAndConstraints,
            "0x" => Self::ZeroX,
            _ => Self::Landing,
        }
    }

    #[must_use]
    pub fn get_page_number(&self) -> &str {
        match self {
            Self::Landing => "00",
            Self::TablesAndConstraints => "01",
            Self::ZeroX => "0x",
        }
    }
}
