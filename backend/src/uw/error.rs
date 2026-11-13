use thiserror::Error;

pub type UwResult<T> = Result<T, UwError>;

#[derive(Debug, Error)]
pub enum UwError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unusual Whales API returned {status}: {body}")]
    Api { status: u16, body: String },

    #[error("failed to deserialize Unusual Whales response: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
}
