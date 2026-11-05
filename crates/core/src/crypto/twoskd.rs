//! Two-Secret Key Derivation: account password + Secret Key -> AUK (spec §4).

use crate::crypto::kdf::{hkdf_expand, KdfParams};
use crate::crypto::{SecretKey, SymmetricKey};
use crate::Result;
use rand::rngs::OsRng;
use rand::TryRngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Domain-separation tag for the password salt (spec §4).
const SALT_INFO: &[u8] = b"2p-salt-v1";

/// Per-account KDF context persisted (without secrets) in `profile.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountContext {
    /// Identifies the account, used as HKDF salt for `p2s` (spec §4).
    pub user_id: String,
    /// Short account id used as HKDF salt for the Secret Key (spec §4).
    pub acct_id: String,
    /// Secret Key version tag, e.g. "P1".
    pub sk_version: String,
    /// KDF choice and its parameters.
    pub kdf: KdfParams,
    /// `p2s`: per-account salt. Not secret.
    #[serde(with = "crate::serde_util::b64_16")]
    pub p2s: [u8; 16],
}

impl AccountContext {
    /// Create a fresh context with a random `p2s` and `acct_id`.
    pub fn new(user_id: impl Into<String>, kdf: KdfParams) -> Self {
        let mut p2s = [0u8; 16];
        OsRng.try_fill_bytes(&mut p2s).expect("OsRng cannot fail");
        let mut acct = [0u8; 4];
        OsRng.try_fill_bytes(&mut acct).expect("OsRng cannot fail");
        Self {
            user_id: user_id.into(),
            acct_id: data_encoding::HEXUPPER.encode(&acct),
            sk_version: "P1".to_string(),
            kdf,
            p2s,
        }
    }
}

/// Derive the Account Unlock Key (spec §4):
///
/// ```text
/// psalt = HKDF(ikm=p2s, salt=user_id, info="2p-salt-v1")
/// pkey  = KDF(password, psalt)
/// skey  = HKDF(ikm=secret_key, salt=acct_id, info=sk_version)
/// AUK   = pkey XOR skey
/// ```
pub fn derive_auk(
    password: &[u8],
    secret_key: &SecretKey,
    ctx: &AccountContext,
) -> Result<SymmetricKey> {
    let psalt = hkdf_expand(&ctx.p2s, ctx.user_id.as_bytes(), SALT_INFO);
    let pkey = ctx.kdf.stretch(password, psalt.as_bytes())?;
    let skey = hkdf_expand(
        secret_key.as_bytes(),
        ctx.acct_id.as_bytes(),
        ctx.sk_version.as_bytes(),
    );

    Ok(SymmetricKey::new(*xor32(pkey.as_bytes(), skey.as_bytes())))
}

/// XOR helper kept `pub(crate)` for tests.
pub(crate) fn xor32(a: &[u8; 32], b: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let mut out = Zeroizing::new([0u8; 32]);
    for i in 0..32 {
        out[i] = a[i] ^ b[i];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_encoding::HEXLOWER;

    fn ctx() -> AccountContext {
        AccountContext {
            user_id: "alice@example.com".into(),
            acct_id: "ABCDEFGH".into(),
            sk_version: "P1".into(),
            kdf: KdfParams::Argon2id { m_kib: 8 * 1024, t: 1, p: 1 },
            p2s: [0x11; 16],
        }
    }

    /// Golden vector from `examples/gen_vectors.rs`. A regression here means
    /// existing vaults can no longer be unlocked — treat as release-blocking.
    #[test]
    fn auk_golden() {
        let sk = SecretKey::from_bytes([0x42; 16]);
        let auk = derive_auk(b"correct horse", &sk, &ctx()).unwrap();
        assert_eq!(
            HEXLOWER.encode(auk.as_bytes()),
            "4f8ebe5d43a2d07341daa9c2fb0fcde84164bc82d02933c582574dc06f641d18"
        );
    }

    #[test]
    fn auk_depends_on_both_secrets() {
        let sk = SecretKey::from_bytes([0x42; 16]);
        let a1 = derive_auk(b"correct horse", &sk, &ctx()).unwrap();
        let a2 = derive_auk(b"wrong horse", &sk, &ctx()).unwrap();
        let a3 = derive_auk(b"correct horse", &SecretKey::from_bytes([0x24; 16]), &ctx()).unwrap();
        assert_ne!(a1.as_bytes(), a2.as_bytes());
        assert_ne!(a1.as_bytes(), a3.as_bytes());
    }
}
