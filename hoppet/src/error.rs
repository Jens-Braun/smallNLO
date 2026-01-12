use std::{error::Error, fmt};

#[derive(Debug)]
pub enum HoppetError {
    AlreadyInitialized,
    #[cfg(feature = "lhapdf")]
    LHAPDFError(lhapdf::Error),
}

impl fmt::Display for HoppetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(
                f,
                "Hoppet is already initialized, re-initializing is only possible if all instances of `Hoppet` are dropped"
            ),
            #[cfg(feature = "lhapdf")]
            Self::LHAPDFError(err) => write!(f, "Error while reading LHAPDF PDF: {err}"),
        }
    }
}

#[cfg(feature = "lhapdf")]
impl From<lhapdf::Error> for HoppetError {
    fn from(value: lhapdf::Error) -> Self {
        return Self::LHAPDFError(value);
    }
}

impl Error for HoppetError {}
