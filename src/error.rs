use std::{error::Error, fmt};

#[derive(Debug)]
pub enum AppError {
    Usage(String),
    Runtime(String),
    Json(serde_json::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => f.write_str(message),
            Self::Json(error) => write!(f, "{error}"),
        }
    }
}

impl Error for AppError {}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
