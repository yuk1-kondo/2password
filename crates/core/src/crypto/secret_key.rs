use crate::{Error, Result};
use rand::rngs::OsRng;
use rand::TryRngCore;
use std::fmt;
use zeroize::Zeroize;

/// The 128-bit Secret Key. Generated on-device at account creation and never
/// leaves the user's devices (spec §4.1). Combined with the account password
/// by 2SKD so that vault data stolen from cloud sync cannot be brute-forced.
///
/// Display format (spec §12): `P1-` prefix followed by the RFC 4648 base32
/// (no padding) encoding, hyphenated in groups of 5:
/// `P1-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-X`.
#[derive(Zeroize)]
pub struct SecretKey([u8; 16]);

impl SecretKey {
    pub const DISPLAY_VERSION: &'static str = "P1";

    pub fn generate() -> Self {
        let mut buf = [0u8; 16];
        OsRng.try_fill_bytes(&mut buf).expect("OsRng cannot fail");
        Self(buf)
    }

    /// Restore a key from its raw 16-byte form (e.g. scanned setup code).
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Human-transcribable form used for new-device setup (spec §7).
    pub fn to_display(&self) -> String {
        let enc = data_encoding::BASE32_NOPAD.encode(&self.0);
        let grouped: Vec<&str> = enc
            .as_bytes()
            .chunks(5)
            .map(|c| std::str::from_utf8(c).expect("base32 is ascii"))
            .collect();
        format!("{}-{}", Self::DISPLAY_VERSION, grouped.join("-"))
    }

    /// Parse a display string back into a `SecretKey`. Accepts with or
    /// without the `P1-` prefix and hyphens; case-insensitive.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim().to_ascii_uppercase();
        let s = s
            .strip_prefix(Self::DISPLAY_VERSION)
            .map(|r| r.trim_start_matches('-'))
            .unwrap_or(&s);
        let compact: String = s.chars().filter(|c| *c != '-').collect();
        let bytes = data_encoding::BASE32_NOPAD
            .decode(compact.as_bytes())
            .map_err(|_| Error::InvalidSecretKey)?;
        let arr: [u8; 16] = bytes.try_into().map_err(|_| Error::InvalidSecretKey)?;
        Ok(Self(arr))
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_roundtrip() {
        let sk = SecretKey::generate();
        let s = sk.to_display();
        assert!(s.starts_with("P1-"));
        let back = SecretKey::parse(&s).unwrap();
        assert_eq!(back.as_bytes(), sk.as_bytes());
    }

    #[test]
    fn parse_accepts_variants() {
        let sk = SecretKey::generate();
        let s = sk.to_display();
        // lowercase, no hyphens, no prefix
        let compact: String = s[3..].chars().filter(|c| *c != '-').collect();
        let back = SecretKey::parse(&compact.to_lowercase()).unwrap();
        assert_eq!(back.as_bytes(), sk.as_bytes());
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(SecretKey::parse("P1-!!!").is_err());
        assert!(SecretKey::parse("P1-AAAA").is_err());
    }
}
