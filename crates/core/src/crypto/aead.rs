//! Authenticated encryption: XChaCha20-Poly1305 with random 192-bit nonces
//! (spec §3). Ciphertext layout: `nonce (24B) || ciphertext+tag`.

use crate::crypto::SymmetricKey;
use crate::{Error, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::rngs::OsRng;
use rand::TryRngCore;
use zeroize::Zeroizing;

pub const NONCE_LEN: usize = 24;
pub const TAG_LEN: usize = 16;

pub fn encrypt(key: &SymmetricKey, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.try_fill_bytes(&mut nonce).expect("OsRng cannot fail");
    let ct = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| Error::Encrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt `nonce || ciphertext+tag`. Plaintext is returned zeroizing-wrapped.
pub fn decrypt(key: &SymmetricKey, data: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if data.len() < NONCE_LEN + TAG_LEN {
        return Err(Error::Decrypt);
    }
    let (nonce, ct) = data.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let pt = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload { msg: ct, aad },
        )
        .map_err(|_| Error::Decrypt)?;
    Ok(Zeroizing::new(pt))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SymmetricKey {
        SymmetricKey::new([0xAA; 32])
    }

    #[test]
    fn roundtrip() {
        let ct = encrypt(&key(), b"hello vault", b"aad").unwrap();
        let pt = decrypt(&key(), &ct, b"aad").unwrap();
        assert_eq!(&*pt, b"hello vault");
    }

    #[test]
    fn tamper_detected() {
        let mut ct = encrypt(&key(), b"hello vault", b"aad").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 1;
        assert!(decrypt(&key(), &ct, b"aad").is_err());
    }

    #[test]
    fn wrong_aad_fails() {
        let ct = encrypt(&key(), b"hello vault", b"aad").unwrap();
        assert!(decrypt(&key(), &ct, b"different").is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let ct = encrypt(&key(), b"hello vault", b"aad").unwrap();
        assert!(decrypt(&SymmetricKey::new([0xBB; 32]), &ct, b"aad").is_err());
    }

    #[test]
    fn short_input_fails() {
        assert!(decrypt(&key(), &[0u8; 10], b"").is_err());
    }
}
