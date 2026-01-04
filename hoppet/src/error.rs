use std::{error::Error, fmt};

#[derive(Debug)]
pub enum HoppetError {
    AlreadyInitialized,
}

impl fmt::Display for HoppetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(
                f,
                "Hoppet is already initialized, re-initializing is only possible if all instances of `Hoppet` are dropped"
            ),
        }
    }
}

impl Error for HoppetError {}
