use std::num::ParseIntError;
use thiserror::Error;

pub mod protodep;
pub mod protofetch;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Parse error")]
    Parse(#[from] ParseIntError),
    #[error("Missing TOML key `{0}` while parsing")]
    MissingKey(String),
    #[error("Missing url component `{0}` in string `{1}`")]
    MissingUrlComponent(String, String),
}
