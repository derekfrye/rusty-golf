use serde::{Deserialize, Serialize};

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