//! UniFFI facade over twopassword-core. Flat, platform-friendly types only;
//! all secrets flow through this boundary as owned Strings/Records.

use std::sync::Arc;
use twopassword_core::crypto::kdf::KdfParams;
use twopassword_core::crypto::SecretKey;
use twopassword_core::generate::{self, PasswordOptions};
use twopassword_core::vault::{ItemPayload, UnlockedVault};
use uuid::Uuid;

uniffi::setup_scaffolding!();

mod capi;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum VaultError {
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
    #[error("invalid options")]
    InvalidOptions,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("i/o error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Json(String),
}

impl From<twopassword_core::Error> for VaultError {
    fn from(e: twopassword_core::Error) -> Self {
        use twopassword_core::Error as E;
        match e {
            E::Kdf => Self::Kdf,
            E::Encrypt => Self::Encrypt,
            E::Decrypt => Self::Decrypt,
            E::InvalidSecretKey => Self::InvalidSecretKey,
            E::Malformed => Self::Malformed,
            E::InvalidOptions => Self::InvalidOptions,
            E::Json(e) => Self::Json(e.to_string()),
            E::Io(e) => Self::Io(e.to_string()),
        }
    }
}

/// Item as shown in lists (no password material).
#[derive(uniffi::Record)]
pub struct ItemSummary {
    pub uuid: String,
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub favorite: bool,
}

/// Full decrypted item.
#[derive(uniffi::Record)]
pub struct ItemDetail {
    pub uuid: String,
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub totp_seed: Option<String>,
    pub favorite: bool,
}

/// Payload for add/update.
#[derive(uniffi::Record)]
pub struct NewItem {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub totp_seed: Option<String>,
    pub favorite: bool,
}

impl From<NewItem> for ItemPayload {
    fn from(n: NewItem) -> Self {
        Self {
            title: n.title,
            username: n.username,
            password: n.password,
            url: n.url,
            notes: n.notes,
            totp_seed: n.totp_seed,
            favorite: n.favorite,
        }
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, VaultError> {
    Uuid::parse_str(s).map_err(|_| VaultError::NotFound(s.to_string()))
}

/// Create a new vault at `dir` using a caller-provided Secret Key.
/// The platform saves the key to its keystore *first*, then calls this —
/// that order guarantees a mid-flow failure can't leave an unopenable
/// vault whose key was never stored (spec §4.1, §7).
#[uniffi::export]
pub fn create_vault(dir: String, user_id: String, password: String, secret_key: String) -> Result<(), VaultError> {
    let sk = SecretKey::parse(&secret_key)?;
    UnlockedVault::create_with_key(&dir, &user_id, KdfParams::default(), &password, &sk)?;
    Ok(())
}

/// Generate a fresh Secret Key in display form (new-device setup, spec §7).
#[uniffi::export]
pub fn new_secret_key() -> String {
    SecretKey::generate().to_display()
}

#[derive(uniffi::Object)]
pub struct Vault {
    inner: UnlockedVault,
}

#[uniffi::export]
pub fn unlock_vault(dir: String, password: String, secret_key: String) -> Result<Arc<Vault>, VaultError> {
    let sk = SecretKey::parse(&secret_key)?;
    let inner = UnlockedVault::unlock(&dir, &password, &sk)?;
    Ok(Arc::new(Vault { inner }))
}

/// Derive the AUK as base64 so the platform can wrap it behind biometrics
/// (spec §8.2). Callers must wrap-and-store, never persist it in plain form.
#[uniffi::export]
pub fn derive_auk_b64(dir: String, password: String, secret_key: String) -> Result<String, VaultError> {
    let profile = UnlockedVault::read_profile(&dir)?;
    let sk = SecretKey::parse(&secret_key)?;
    let auk = twopassword_core::crypto::twoskd::derive_auk(
        password.as_bytes(),
        &sk,
        &profile.account,
    )?;
    Ok(data_encoding::BASE64_NOPAD.encode(auk.as_bytes()))
}

/// Unlock with a wrapped-then-unwrapped AUK (biometric path, spec §8.2).
#[uniffi::export]
pub fn unlock_vault_with_auk(dir: String, auk_b64: String) -> Result<Arc<Vault>, VaultError> {
    let bytes = data_encoding::BASE64_NOPAD
        .decode(auk_b64.as_bytes())
        .map_err(|_| VaultError::Malformed)?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| VaultError::Malformed)?;
    let auk = twopassword_core::crypto::SymmetricKey::new(arr);
    let inner = UnlockedVault::unlock_with_auk(&dir, &auk)?;
    Ok(Arc::new(Vault { inner }))
}

#[uniffi::export]
impl Vault {
    /// Decrypt all non-tombstone items for listing.
    pub fn list_items(&self) -> Result<Vec<ItemSummary>, VaultError> {
        let mut out = Vec::new();
        for file in self.inner.item_files()? {
            if file.tombstone {
                continue;
            }
            let p = self.inner.decrypt(&file)?;
            out.push(ItemSummary {
                uuid: file.uuid.to_string(),
                title: p.title,
                username: p.username,
                url: p.url,
                favorite: p.favorite,
            });
        }
        Ok(out)
    }

    pub fn get_item(&self, uuid: String) -> Result<ItemDetail, VaultError> {
        let id = parse_uuid(&uuid)?;
        let file = self.inner.load_file(&id)?;
        let p = self.inner.decrypt(&file)?;
        Ok(ItemDetail {
            uuid: file.uuid.to_string(),
            title: p.title,
            username: p.username,
            password: p.password,
            url: p.url,
            notes: p.notes,
            totp_seed: p.totp_seed,
            favorite: p.favorite,
        })
    }

    pub fn add_item(&self, item: NewItem) -> Result<String, VaultError> {
        Ok(self.inner.add(&item.into())?.to_string())
    }

    pub fn update_item(&self, uuid: String, item: NewItem) -> Result<(), VaultError> {
        Ok(self.inner.update(&parse_uuid(&uuid)?, &item.into())?)
    }

    /// Tombstone so the deletion propagates through sync (spec §6).
    pub fn remove_item(&self, uuid: String) -> Result<(), VaultError> {
        Ok(self.inner.tombstone(&parse_uuid(&uuid)?)?)
    }

    /// Re-wrap the keyset under a new password (spec §5). Verify the old
    /// password before calling.
    pub fn change_password(&self, new_password: String, secret_key: String) -> Result<(), VaultError> {
        let sk = SecretKey::parse(&secret_key)?;
        Ok(self.inner.change_password(&new_password, &sk)?)
    }

    /// Import items from a CSV file. Returns count added (spec §9).
    pub fn import_csv(&self, path: String) -> Result<u32, VaultError> {
        Ok(self.inner.import_csv(std::path::Path::new(&path))? as u32)
    }

    /// Export all items to plaintext CSV. Returns count exported (spec §9).
    pub fn export_csv(&self, path: String) -> Result<u32, VaultError> {
        Ok(self.inner.export_csv(std::path::Path::new(&path))? as u32)
    }
}

#[uniffi::export]
pub fn generate_password(
    length: u32,
    digits: bool,
    symbols: bool,
    avoid_ambiguous: bool,
) -> Result<String, VaultError> {
    Ok(generate::password(&PasswordOptions {
        length: length as usize,
        digits,
        symbols,
        avoid_ambiguous,
    })?)
}

#[uniffi::export]
pub fn generate_passphrase(word_count: u32, separator: String) -> Result<String, VaultError> {
    let sep = separator.chars().next().unwrap_or('-');
    Ok(generate::passphrase(word_count as usize, sep)?)
}

#[uniffi::export]
pub fn generate_pin(length: u32) -> Result<String, VaultError> {
    Ok(generate::pin(length as usize)?)
}

/// Normalize a TOTP seed: raw base32 or otpauth:// URI (spec §9).
#[uniffi::export]
pub fn normalize_totp_seed(input: String) -> Result<String, VaultError> {
    Ok(twopassword_core::totp::normalize_seed(&input)?)
}

/// Current TOTP code (6 digits) at the given unix timestamp.
#[uniffi::export]
pub fn totp_code(seed: String, timestamp_secs: u64) -> Result<String, VaultError> {
    Ok(twopassword_core::totp::code(&seed, timestamp_secs)?)
}

/// HIBP breach count via k-anonymity (spec §9). 0 = not seen in breaches.
#[uniffi::export]
pub fn check_password_breach(password: String) -> Result<u32, VaultError> {
    Ok(twopassword_core::hibp::breach_count(&password)?)
}

/// eTLD+1 domain match for autofill (spec §8.3). `item_url` may be nil.
#[uniffi::export]
pub fn domain_matches(item_url: Option<String>, host: String) -> bool {
    twopassword_core::domain::matches(item_url.as_deref(), &host)
}
