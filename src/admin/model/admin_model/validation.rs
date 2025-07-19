use regex::Regex;

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
        AlphaNum14::new(value)
            .ok_or("Invalid alphanumeric string: must be exactly 14 alphanumeric characters")
    }
}

impl std::fmt::Display for AlphaNum14 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AlphaNum14 {
    /// # Panics
    ///
    /// Will panic if the regex is invalid
    #[must_use]
    pub fn new(input: &str) -> Option<Self> {
        // Using a static regex for better performance and safety
        use std::sync::OnceLock;
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let re = REGEX.get_or_init(|| {
            Regex::new(r"^[a-zA-Z0-9]{14}$")
                .expect("Invalid regex pattern - this is a programming error")
        });

        if re.is_match(input) {
            Some(AlphaNum14(input.to_string()))
        } else {
            None
        }
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }

    /// # Errors
    ///
    /// Will return `Err` if the input is not a 14 character alphanumeric string
    pub fn parse(input: &str) -> Result<Self, String> {
        Self::try_from(input).map_err(|_| "Invalid input".to_string())
    }
}
