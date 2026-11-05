//! Print deterministic crypto vectors used by the unit tests.
//! Run: `cargo run -p twopassword-core --example gen_vectors`

use data_encoding::HEXLOWER;
use twopassword_core::crypto::kdf::{hkdf_expand, KdfParams};
use twopassword_core::crypto::twoskd::{derive_auk, AccountContext};
use twopassword_core::crypto::SecretKey;

fn main() {
    let argon = KdfParams::Argon2id {
        m_kib: 8 * 1024,
        t: 1,
        p: 1,
    };
    let k = argon.stretch(b"password", b"0123456789abcdef").unwrap();
    println!("argon2id_small = {}", HEXLOWER.encode(k.as_bytes()));

    let pbkdf = KdfParams::Pbkdf2Sha256 { iterations: 1000 };
    let k2 = pbkdf.stretch(b"password", b"0123456789abcdef").unwrap();
    println!("pbkdf2_1000    = {}", HEXLOWER.encode(k2.as_bytes()));

    let ctx = AccountContext {
        user_id: "alice@example.com".into(),
        acct_id: "ABCDEFGH".into(),
        sk_version: "P1".into(),
        kdf: argon,
        p2s: [0x11; 16],
    };
    let sk = SecretKey::from_bytes([0x42; 16]);
    let auk = derive_auk(b"correct horse", &sk, &ctx).unwrap();
    println!("auk            = {}", HEXLOWER.encode(auk.as_bytes()));

    let hk = hkdf_expand(b"ikm-data", b"salt", b"test-info");
    println!("hkdf           = {}", HEXLOWER.encode(hk.as_bytes()));
}
