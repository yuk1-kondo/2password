//! twopassword-core: shared cryptographic and vault logic for all 2Password
//! clients (macOS, iOS, Windows, Android).
//!
//! Security design: see `docs/security-spec.md`.

pub mod crypto;
pub mod error;
pub mod generate;
pub mod import_export;
mod serde_util;
pub mod domain;
pub mod hibp;
pub mod totp;
pub mod vault;

pub use error::{Error, Result};
