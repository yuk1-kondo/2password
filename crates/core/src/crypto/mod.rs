//! Cryptographic primitives (spec sections 3–5).

pub mod aead;
pub mod kdf;
pub mod secret_key;
pub mod twoskd;

pub use secret_key::SecretKey;

use rand::rngs::OsRng;
use rand::TryRngCore;
use zeroize::Zeroizing;

/// A 256-bit symmetric key that is zeroed on drop.
pub struct SymmetricKey(Zeroizing<[u8; 32]>);

impl SymmetricKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Generate a fresh key from the OS CSPRNG (spec §3).
    pub fn generate() -> Self {
        let mut buf = [0u8; 32];
        OsRng.try_fill_bytes(&mut buf).expect("OsRng cannot fail");
        Self::new(buf)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
