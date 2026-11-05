//! RFC 6238 TOTP (HMAC-SHA1, 30s period, 6 digits) — spec §9.

use crate::{Error, Result};
use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

const PERIOD: u64 = 30;
const DIGITS: u32 = 6;

/// Decode a base32 seed, tolerating spaces, lowercase and padding.
fn decode_seed(input: &str) -> Result<Vec<u8>> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    let cleaned = cleaned.trim_end_matches('=');
    data_encoding::BASE32_NOPAD
        .decode(cleaned.as_bytes())
        .map_err(|_| Error::Malformed)
}

/// Normalize a user-supplied seed. Accepts raw base32 or an
/// `otpauth://totp/...?secret=...` URI; returns canonical base32.
pub fn normalize_seed(input: &str) -> Result<String> {
    let s = input.trim();
    let candidate = if let Some(rest) = s.strip_prefix("otpauth://") {
        rest.split(['?', '&'])
            .find_map(|p| p.strip_prefix("secret="))
            .ok_or(Error::Malformed)?
            .to_string()
    } else {
        s.to_string()
    };
    let key = decode_seed(&candidate)?;
    Ok(data_encoding::BASE32_NOPAD.encode(&key))
}

/// Compute the 6-digit TOTP code for `timestamp_secs` (unix seconds).
pub fn code(seed: &str, timestamp_secs: u64) -> Result<String> {
    let key = decode_seed(seed)?;
    let counter = timestamp_secs / PERIOD;
    let mut mac = HmacSha1::new_from_slice(&key).map_err(|_| Error::Kdf)?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | (digest[offset + 3] as u32);
    Ok(format!("{:0width$}", binary % 10u32.pow(DIGITS), width = DIGITS as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 Appendix B SHA-1 test key ("12345678901234567890" ASCII).
    const SEED: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn rfc6238_vectors() {
        // 8-digit expected values truncated to 6 digits.
        assert_eq!(code(SEED, 59).unwrap(), "287082");
        assert_eq!(code(SEED, 1111111109).unwrap(), "081804");
        assert_eq!(code(SEED, 1234567890).unwrap(), "005924");
    }

    #[test]
    fn normalizes_variants() {
        let lower = SEED.to_lowercase();
        assert_eq!(normalize_seed(&lower).unwrap(), SEED);
        assert_eq!(normalize_seed(" gezd gnbv gy3t qojq gezd gnbv gy3t qojq ").unwrap(), SEED);
        assert_eq!(
            normalize_seed("otpauth://totp/a@b.c?secret=gezdgnbvgy3tqojqgezdgnbvgy3tqojq&issuer=X").unwrap(),
            SEED
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(normalize_seed("not-base32!!!").is_err());
        assert!(normalize_seed("otpauth://totp/a?issuer=X").is_err());
    }
}
