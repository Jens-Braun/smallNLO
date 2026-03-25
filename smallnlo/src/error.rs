use std::fmt::Debug;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Unable to read fastNLO table")]
    IOError(#[from] std::io::Error),
    #[error("Invalid table format: expected `{0}`, found `{1}`")]
    FormatError(String, String),
    #[error("Unexpected end of file, expected {0}")]
    UnexpectedEOFError(String),
    #[error("Error while parsing int")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Error while parsing float")]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("Error while parsing bool")]
    ParseBoolError(#[from] std::str::ParseBoolError),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
}

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("Unable to write to output file")]
    IOError(#[from] std::io::Error),
    #[error("Unable to write to output stream")]
    FmtError(#[from] std::fmt::Error),
}

#[derive(Error, Debug)]
pub enum MergeError {
    #[error("Metadata field {0} is incompatible: {1} vs {2}")]
    IncompatibleTables(String, String, String),
}
