use serde::{Serialize, Serializer};

/// Unified error type for the whole application.
///
/// It serializes to a plain string so it can be returned directly from
/// `#[tauri::command]` functions and shown in the frontend / CLI.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid config: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("could not serialize config: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("no capture backend is available for this session")]
    NoBackend,

    #[error("capture was cancelled")]
    Cancelled,

    #[error("backend '{0}' is unavailable: {1}")]
    BackendUnavailable(String, String),

    #[error("could not parse region geometry: {0}")]
    Geometry(String),

    #[error("capture produced no data")]
    EmptyCapture,

    #[error("network error: {0}")]
    Http(String),

    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Error::Message(message.into())
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
