//! Per-item files: `items/<uuid>.json` (spec §6).

use crate::crypto::{aead, SymmetricKey};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// The encrypted payload of an item. Everything user-visible lives inside
/// this payload; the file only exposes uuid/updated_at/tombstone (spec §6).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ItemPayload {
    #[serde(rename = "t")]
    pub title: String,
    #[serde(rename = "u", default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(rename = "p", default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "l", default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "n", default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// RFC 6238 TOTP seed (spec §9), if attached.
    #[serde(rename = "totp", default, skip_serializing_if = "Option::is_none")]
    pub totp_seed: Option<String>,
    #[serde(rename = "fav", default)]
    pub favorite: bool,
}

/// `items/<uuid>.json` on disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemFile {
    pub uuid: Uuid,
    pub v: u32,
    /// base64(nonce || ciphertext+tag)
    #[serde(with = "crate::serde_util::b64_vec")]
    pub ct: Vec<u8>,
    /// The AAD string, recorded for tamper-evidence auditing (spec §6).
    pub aad: String,
    #[serde(default)]
    pub tombstone: bool,
    /// Unix seconds. Exposed unencrypted for sync conflict resolution.
    pub updated_at: u64,
}

fn aad_string(uuid: &Uuid, vault_id: &Uuid, v: u32) -> String {
    format!("{uuid}|{vault_id}|{v}")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl ItemFile {
    /// Create a new encrypted item.
    pub fn create(vault_key: &SymmetricKey, vault_id: &Uuid, payload: &ItemPayload) -> Result<Self> {
        let uuid = Uuid::new_v4();
        Self::update(vault_key, vault_id, payload, uuid, 1, false)
    }

    /// Re-encrypt an existing item (new contents or tombstone change).
    pub fn update(
        vault_key: &SymmetricKey,
        vault_id: &Uuid,
        payload: &ItemPayload,
        uuid: Uuid,
        v: u32,
        tombstone: bool,
    ) -> Result<Self> {
        let aad = aad_string(&uuid, vault_id, v);
        let ct = aead::encrypt(vault_key, &serde_json::to_vec(payload)?, aad.as_bytes())?;
        Ok(Self {
            uuid,
            v,
            ct,
            aad,
            tombstone,
            updated_at: now_unix(),
        })
    }

    /// Verify the recorded AAD and decrypt the payload.
    pub fn decrypt(&self, vault_key: &SymmetricKey, vault_id: &Uuid) -> Result<ItemPayload> {
        if self.aad != aad_string(&self.uuid, vault_id, self.v) {
            return Err(Error::Malformed);
        }
        let pt = aead::decrypt(vault_key, &self.ct, self.aad.as_bytes())?;
        Ok(serde_json::from_slice(&pt)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (SymmetricKey, Uuid, ItemPayload) {
        let payload = ItemPayload {
            title: "Example".into(),
            username: Some("alice".into()),
            password: Some("s3cret".into()),
            url: Some("https://example.com".into()),
            ..Default::default()
        };
        (SymmetricKey::generate(), Uuid::new_v4(), payload)
    }

    #[test]
    fn create_decrypt_roundtrip() {
        let (key, vault_id, payload) = fixture();
        let file = ItemFile::create(&key, &vault_id, &payload).unwrap();
        let back = file.decrypt(&key, &vault_id).unwrap();
        assert_eq!(back.title, "Example");
        assert_eq!(back.password.as_deref(), Some("s3cret"));
    }

    #[test]
    fn tampered_aad_fails() {
        let (key, vault_id, payload) = fixture();
        let mut file = ItemFile::create(&key, &vault_id, &payload).unwrap();
        file.aad = format!("{}|{}|{}", file.uuid, Uuid::new_v4(), file.v);
        assert!(matches!(file.decrypt(&key, &vault_id), Err(Error::Malformed)));
    }

    #[test]
    fn wrong_vault_fails() {
        let (key, vault_id, payload) = fixture();
        let file = ItemFile::create(&key, &vault_id, &payload).unwrap();
        assert!(file.decrypt(&key, &Uuid::new_v4()).is_err());
    }

    #[test]
    fn tampered_ct_fails() {
        let (key, vault_id, payload) = fixture();
        let mut file = ItemFile::create(&key, &vault_id, &payload).unwrap();
        let last = file.ct.len() - 1;
        file.ct[last] ^= 1;
        assert!(file.decrypt(&key, &vault_id).is_err());
    }
}
