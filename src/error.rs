use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("altium error: {0}")]
    Altium(#[from] altium::Error),

    #[error("EasyEDA API: {0}")]
    Api(String),

    #[error("EasyEDA shape parse: {0}")]
    Parse(String),

    #[error("conversion: {0}")]
    Convert(String),

    #[error("output file already exists: {0} (use --overwrite)")]
    AlreadyExists(String),

    #[error("invalid argument: {0}")]
    InvalidArg(String),
}

pub type Result<T> = std::result::Result<T, Error>;
