use crate::crypto::kdf::KdfParams;
use crate::crypto::twoskd::AccountContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `profile.json` — non-secret account metadata (spec §6).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    /// File format version.
    pub version: u32,
    /// Crypto suite tag (spec §3).
    pub crypto: String,
    pub vault_id: Uuid,
    #[serde(flatten)]
    pub account: AccountContext,
}

impl Profile {
    pub const CRYPTO_SUITE: &'static str = "xchacha20poly1305-v1";

    pub fn new(user_id: impl Into<String>, kdf: KdfParams) -> Self {
        Self {
            version: 1,
            crypto: Self::CRYPTO_SUITE.to_string(),
            vault_id: Uuid::new_v4(),
            account: AccountContext::new(user_id, kdf),
        }
    }
}
