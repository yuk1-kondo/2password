//! Vault directory I/O: `profile.json`, `keyset.json`, `items/` (spec §6).
//! This is the layer every platform client drives through FFI.

use super::item::ItemFile;
use super::keyset::WrappedKeyset;
use super::{ItemPayload, Keyset, Profile};
use crate::crypto::kdf::KdfParams;
use crate::crypto::twoskd::derive_auk;
use crate::crypto::{SecretKey, SymmetricKey};
use crate::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PROFILE_FILE: &str = "profile.json";
const KEYSET_FILE: &str = "keyset.json";
const ITEMS_DIR: &str = "items";

/// Last-write-wins merge by `updated_at` (spec §7). Tombstones compete on
/// the same clock, so a delete beats an older edit.
fn dedupe_items(files: Vec<ItemFile>) -> Vec<ItemFile> {
    let mut by_uuid: std::collections::BTreeMap<Uuid, ItemFile> =
        std::collections::BTreeMap::new();
    for f in files {
        by_uuid
            .entry(f.uuid)
            .and_modify(|e| {
                if f.updated_at > e.updated_at {
                    *e = f.clone();
                }
            })
            .or_insert(f);
    }
    by_uuid.into_values().collect()
}

/// A vault unlocked in memory. Dropping it zeroizes the keyset; there is no
/// shared secret state left behind (spec §8.2).
pub struct UnlockedVault {
    dir: PathBuf,
    profile: Profile,
    keyset: Keyset,
}

impl UnlockedVault {
    /// Initialize a new vault directory, generating a fresh Secret Key.
    /// Convenience wrapper around [`create_with_key`].
    pub fn create(
        dir: impl AsRef<Path>,
        user_id: &str,
        kdf: KdfParams,
        password: &str,
    ) -> Result<SecretKey> {
        let secret_key = SecretKey::generate();
        Self::create_with_key(dir, user_id, kdf, password, &secret_key)?;
        Ok(secret_key)
    }

    /// Initialize a new vault directory with a caller-provided Secret Key.
    /// Lets callers persist the key in the platform keystore *before* the
    /// vault exists, so a later failure can't orphan an unopenable vault.
    pub fn create_with_key(
        dir: impl AsRef<Path>,
        user_id: &str,
        kdf: KdfParams,
        password: &str,
        secret_key: &SecretKey,
    ) -> Result<()> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir.join(ITEMS_DIR))?;
        let profile = Profile::new(user_id, kdf);
        let auk = derive_auk(password.as_bytes(), secret_key, &profile.account)?;
        let keyset = Keyset::generate_for(profile.vault_id);
        fs::write(
            dir.join(PROFILE_FILE),
            serde_json::to_string_pretty(&profile)?,
        )?;
        fs::write(
            dir.join(KEYSET_FILE),
            serde_json::to_string_pretty(&keyset.wrap(&auk)?)?,
        )?;
        Ok(())
    }

    /// Read `profile.json` without unlocking (needed to derive the AUK).
    pub fn read_profile(dir: impl AsRef<Path>) -> Result<Profile> {
        Ok(serde_json::from_str(
            &fs::read_to_string(dir.as_ref().join(PROFILE_FILE))?,
        )?)
    }

    /// Unlock an existing vault with password + Secret Key (2SKD, spec §4).
    pub fn unlock(
        dir: impl AsRef<Path>,
        password: &str,
        secret_key: &SecretKey,
    ) -> Result<Self> {
        let dir = dir.as_ref();
        let profile = Self::read_profile(dir)?;
        let auk = derive_auk(password.as_bytes(), secret_key, &profile.account)?;
        Self::unlock_with_auk(dir, &auk)
    }

    /// Unlock with a previously derived AUK — the biometric-unlock path
    /// (spec §8.2). The caller is responsible for gating this on hardware
    /// authentication (Keychain access control / Keystore CryptoObject).
    pub fn unlock_with_auk(dir: impl AsRef<Path>, auk: &SymmetricKey) -> Result<Self> {
        let dir = dir.as_ref();
        let profile = Self::read_profile(dir)?;
        let wrapped: WrappedKeyset =
            serde_json::from_str(&fs::read_to_string(dir.join(KEYSET_FILE))?)?;
        let keyset = Keyset::unwrap(&wrapped, auk)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            profile,
            keyset,
        })
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    fn vault_key(&self) -> Result<&SymmetricKey> {
        self.keyset
            .vault_key(&self.profile.vault_id)
            .ok_or(Error::Malformed)
    }

    fn item_path(&self, uuid: &Uuid) -> PathBuf {
        self.dir.join(ITEMS_DIR).join(format!("{uuid}.json"))
    }

    /// Encrypt and store a new item. Returns its uuid.
    pub fn add(&self, payload: &ItemPayload) -> Result<Uuid> {
        let file = ItemFile::create(self.vault_key()?, &self.profile.vault_id, payload)?;
        fs::write(
            self.item_path(&file.uuid),
            serde_json::to_string_pretty(&file)?,
        )?;
        Ok(file.uuid)
    }

    /// All item files including tombstones (sync needs them, spec §6).
    /// File-sync providers can leave conflict copies of the same item;
    /// dedupe by uuid keeping the newest `updated_at` (spec §7, LWW).
    pub fn item_files(&self) -> Result<Vec<ItemFile>> {
        let mut files = Vec::new();
        let items_dir = self.dir.join(ITEMS_DIR);
        if !items_dir.exists() {
            return Ok(files);
        }
        for entry in fs::read_dir(items_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(serde_json::from_str(&fs::read_to_string(&path)?)?);
            }
        }
        Ok(dedupe_items(files))
    }

    pub fn load_file(&self, uuid: &Uuid) -> Result<ItemFile> {
        Ok(serde_json::from_str(
            &fs::read_to_string(self.item_path(uuid))?,
        )?)
    }

    /// Decrypt an item file into its payload.
    pub fn decrypt(&self, file: &ItemFile) -> Result<ItemPayload> {
        file.decrypt(self.vault_key()?, &self.profile.vault_id)
    }

    /// Re-encrypt an existing item in place (bumping `updated_at`).
    pub fn update(&self, uuid: &Uuid, payload: &ItemPayload) -> Result<()> {
        let file = ItemFile::update(
            self.vault_key()?,
            &self.profile.vault_id,
            payload,
            *uuid,
            1,
            false,
        )?;
        fs::write(self.item_path(uuid), serde_json::to_string_pretty(&file)?)?;
        Ok(())
    }

    /// Import items from a CSV file (spec §9). Returns count added.
    pub fn import_csv(&self, path: &Path) -> Result<usize> {
        let items = crate::import_export::import_csv(path)?;
        let n = items.len();
        for it in &items {
            self.add(it)?;
        }
        Ok(n)
    }

    /// Export all non-tombstone items to plaintext CSV (spec §9 — caller must
    /// warn the user). Returns count exported.
    pub fn export_csv(&self, path: &Path) -> Result<usize> {
        let mut items = Vec::new();
        for f in self.item_files()? {
            if !f.tombstone {
                items.push(self.decrypt(&f)?);
            }
        }
        crate::import_export::export_csv(path, &items)?;
        Ok(items.len())
    }

    /// Change the account password. Only re-wraps the keyset under a new
    /// AUK — items stay untouched (spec §5). Caller must verify the old
    /// password first (e.g. by attempting `unlock`).
    pub fn change_password(&self, new_password: &str, secret_key: &SecretKey) -> Result<()> {
        let auk = derive_auk(new_password.as_bytes(), secret_key, &self.profile.account)?;
        fs::write(
            self.dir.join(KEYSET_FILE),
            serde_json::to_string_pretty(&self.keyset.wrap(&auk)?)?,
        )?;
        Ok(())
    }

    /// Tombstone an item so the deletion propagates through sync (spec §6).
    pub fn tombstone(&self, uuid: &Uuid) -> Result<()> {
        let file = ItemFile::update(
            self.vault_key()?,
            &self.profile.vault_id,
            &ItemPayload::default(),
            *uuid,
            1,
            true,
        )?;
        fs::write(self.item_path(uuid), serde_json::to_string_pretty(&file)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(uuid: Uuid, updated_at: u64, tombstone: bool) -> ItemFile {
        ItemFile {
            uuid,
            v: 1,
            ct: vec![],
            aad: String::new(),
            tombstone,
            updated_at,
        }
    }

    #[test]
    fn dedupe_keeps_newest() {
        let id = Uuid::new_v4();
        let out = dedupe_items(vec![
            file(id, 100, false),
            file(id, 200, true), // conflict copy wins
        ]);
        assert_eq!(out.len(), 1);
        assert!(out[0].tombstone);
        assert_eq!(out[0].updated_at, 200);
    }

    #[test]
    fn dedupe_preserves_distinct() {
        let out = dedupe_items(vec![
            file(Uuid::new_v4(), 1, false),
            file(Uuid::new_v4(), 2, false),
        ]);
        assert_eq!(out.len(), 2);
    }
}
