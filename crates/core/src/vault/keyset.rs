//! Master keyset: wraps the per-vault keys with the AUK (spec §5).

use crate::crypto::{aead, SymmetricKey};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Domain-separation AAD for the wrapped keyset blob.
const KEYSET_AAD: &[u8] = b"keyset-v1";

/// The master keyset. Exists only in unlocked memory; at rest it is stored
/// as `keyset.json`, a single XChaCha20-Poly1305 blob under the AUK.
pub struct Keyset {
    keys: BTreeMap<Uuid, SymmetricKey>,
}

#[derive(Serialize, Deserialize)]
struct KeysetPlaintext {
    /// vault_id -> base64 vault key
    vaults: BTreeMap<Uuid, String>,
}

/// `keyset.json` on disk.
#[derive(Serialize, Deserialize)]
pub struct WrappedKeyset {
    pub v: u32,
    #[serde(with = "crate::serde_util::b64_vec")]
    pub ct: Vec<u8>,
}

impl Keyset {
    /// Create a keyset containing a fresh vault key for `vault_id`.
    pub fn generate_for(vault_id: Uuid) -> Self {
        let mut keys = BTreeMap::new();
        keys.insert(vault_id, SymmetricKey::generate());
        Self { keys }
    }

    pub fn vault_key(&self, vault_id: &Uuid) -> Option<&SymmetricKey> {
        self.keys.get(vault_id)
    }

    /// Add a fresh key for an additional vault (spec §5).
    pub fn add_vault(&mut self, vault_id: Uuid) {
        self.keys.insert(vault_id, SymmetricKey::generate());
    }

    /// Encrypt under the AUK for storage as `keyset.json`.
    pub fn wrap(&self, auk: &SymmetricKey) -> Result<WrappedKeyset> {
        let pt = KeysetPlaintext {
            vaults: self
                .keys
                .iter()
                .map(|(id, k)| (*id, data_encoding::BASE64_NOPAD.encode(k.as_bytes())))
                .collect(),
        };
        let ct = aead::encrypt(auk, &serde_json::to_vec(&pt)?, KEYSET_AAD)?;
        Ok(WrappedKeyset { v: 1, ct })
    }

    /// Decrypt `keyset.json` with the AUK.
    pub fn unwrap(wrapped: &WrappedKeyset, auk: &SymmetricKey) -> Result<Self> {
        if wrapped.v != 1 {
            return Err(Error::Malformed);
        }
        let pt = aead::decrypt(auk, &wrapped.ct, KEYSET_AAD)?;
        let decoded: KeysetPlaintext = serde_json::from_slice(&pt)?;
        let mut keys = BTreeMap::new();
        for (id, b64) in decoded.vaults {
            let bytes = data_encoding::BASE64_NOPAD
                .decode(b64.as_bytes())
                .map_err(|_| Error::Malformed)?;
            let arr: [u8; 32] = bytes.try_into().map_err(|_| Error::Malformed)?;
            keys.insert(id, SymmetricKey::new(arr));
        }
        Ok(Self { keys })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_roundtrip() {
        let vault_id = Uuid::new_v4();
        let keyset = Keyset::generate_for(vault_id);
        let auk = SymmetricKey::generate();
        let wrapped = keyset.wrap(&auk).unwrap();
        let back = Keyset::unwrap(&wrapped, &auk).unwrap();
        assert_eq!(
            keyset.vault_key(&vault_id).unwrap().as_bytes(),
            back.vault_key(&vault_id).unwrap().as_bytes()
        );
    }

    #[test]
    fn wrong_auk_fails() {
        let keyset = Keyset::generate_for(Uuid::new_v4());
        let wrapped = keyset.wrap(&SymmetricKey::generate()).unwrap();
        assert!(Keyset::unwrap(&wrapped, &SymmetricKey::generate()).is_err());
    }

    #[test]
    fn json_roundtrip() {
        let keyset = Keyset::generate_for(Uuid::new_v4());
        let auk = SymmetricKey::generate();
        let json = serde_json::to_string(&keyset.wrap(&auk).unwrap()).unwrap();
        let parsed: WrappedKeyset = serde_json::from_str(&json).unwrap();
        assert!(Keyset::unwrap(&parsed, &auk).is_ok());
    }
}
