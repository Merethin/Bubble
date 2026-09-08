mod rule;
mod parser;

use std::{error::Error, fmt::Display};

pub use rule::ParsedRule;
pub use parser::parse_config;

#[derive(Debug)]
pub(crate) struct ConfigError { pub message: String }
impl Error for ConfigError {}
impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConfigError: {}", self.message)
    }
}