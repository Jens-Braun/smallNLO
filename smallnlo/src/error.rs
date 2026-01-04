use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Unable to read fastNLO table")]
    IOError(#[from] std::io::Error),
    #[error("Invalid table format: expected `{0}`, found `{1}`")]
    FormatError(String, String),
    #[error("Unexpected end of file, expected {0}")]
    UnexpectedEOFError(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    ParseBoolError(#[from] std::str::ParseBoolError),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
}
