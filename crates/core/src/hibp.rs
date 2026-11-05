//! Have I Been Pwned breach check via k-anonymity (spec §9):
//! only the first 5 chars of SHA-1(password) leave the device.

use crate::{Error, Result};
use sha1::{Digest, Sha1};

/// Number of times `password` appears in public breaches. 0 = not seen.
/// Errors are network/parse failures, not "password is bad".
pub fn breach_count(password: &str) -> Result<u32> {
    let hash = format!("{:X}", Sha1::digest(password.as_bytes()));
    let (prefix, suffix) = hash.split_at(5);
    let body = ureq::get(&format!("https://api.pwnedpasswords.com/range/{prefix}"))
        .header("User-Agent", "2password-breach-check")
        .call()
        .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    for line in body.lines() {
        if let Some((suf, count)) = line.trim().split_once(':') {
            if suf.eq_ignore_ascii_case(suffix) {
                return count.parse().map_err(|_| Error::Malformed);
            }
        }
    }
    Ok(0)
}
