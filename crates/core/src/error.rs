use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("key derivation failed")]
    Kdf,
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
    #[error("invalid secret key format")]
    InvalidSecretKey,
    #[error("malformed vault file")]
    Malformed,
    #[error("invalid generator options")]
    InvalidOptions,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
