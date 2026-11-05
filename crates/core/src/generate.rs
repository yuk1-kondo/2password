//! CSPRNG-backed password / passphrase / PIN generation (spec §9).
//! All generation happens in the core so every platform behaves identically.

use crate::{Error, Result};
use rand::seq::IndexedRandom;
use rand::Rng;
use std::sync::OnceLock;

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.<>?";
/// Characters frequently confused: l I 1 O 0
const AMBIGUOUS: &[u8] = b"lI1O0";

#[derive(Clone, Copy, Debug)]
pub struct PasswordOptions {
    pub length: usize,
    pub digits: bool,
    pub symbols: bool,
    /// Exclude easily-confused characters (l I 1 O 0).
    pub avoid_ambiguous: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 20,
            digits: true,
            symbols: true,
            avoid_ambiguous: false,
        }
    }
}

fn strip_ambiguous(set: &[u8]) -> Vec<u8> {
    set.iter().copied().filter(|c| !AMBIGUOUS.contains(c)).collect()
}

/// Generate a random password. Every enabled character class is guaranteed
/// to appear at least once (rejection sampling).
pub fn password(opts: &PasswordOptions) -> Result<String> {
    if opts.length == 0 || opts.length > 256 {
        return Err(Error::InvalidOptions);
    }
    let mut rng = rand::rng();
    let mut classes: Vec<Vec<u8>> = Vec::new();
    let mut pool: Vec<u8> = Vec::new();
    for set in [
        Some(LOWER.to_vec()),
        Some(UPPER.to_vec()),
        opts.digits.then(|| DIGITS.to_vec()),
        opts.symbols.then(|| SYMBOLS.to_vec()),
    ]
    .into_iter()
    .flatten()
    {
        let set = if opts.avoid_ambiguous {
            strip_ambiguous(&set)
        } else {
            set
        };
        pool.extend_from_slice(&set);
        classes.push(set);
    }
    if opts.length < classes.len() {
        return Err(Error::InvalidOptions);
    }
    loop {
        let pw: Vec<u8> = (0..opts.length)
            .map(|_| *pool.choose(&mut rng).expect("pool non-empty"))
            .collect();
        if classes.iter().all(|c| pw.iter().any(|ch| c.contains(ch))) {
            return Ok(String::from_utf8(pw).expect("charset is ascii"));
        }
    }
}

/// Numeric PIN of `len` digits.
pub fn pin(len: usize) -> Result<String> {
    if len == 0 || len > 64 {
        return Err(Error::InvalidOptions);
    }
    let mut rng = rand::rng();
    Ok((0..len)
        .map(|_| (b'0' + rng.random_range(0..10)) as char)
        .collect())
}

fn words() -> &'static Vec<&'static str> {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        include_str!("../assets/eff_large_wordlist.txt")
            .lines()
            .filter_map(|l| l.split('\t').nth(1))
            .map(str::trim)
            .collect()
    })
}

/// Diceware-style passphrase from the EFF large wordlist (spec §9).
/// `words` words joined by `sep`, e.g. `correct-horse-battery-staple`.
pub fn passphrase(word_count: usize, sep: char) -> Result<String> {
    if word_count == 0 || word_count > 20 {
        return Err(Error::InvalidOptions);
    }
    let words = words();
    let mut rng = rand::rng();
    Ok((0..word_count)
        .map(|_| *words.choose(&mut rng).expect("wordlist non-empty"))
        .collect::<Vec<_>>()
        .join(&sep.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordlist_loads_7776() {
        assert_eq!(words().len(), 7776);
    }

    #[test]
    fn password_has_all_classes() {
        let pw = password(&PasswordOptions::default()).unwrap();
        assert_eq!(pw.len(), 20);
        assert!(pw.bytes().any(|c| DIGITS.contains(&c)));
        assert!(pw.bytes().any(|c| SYMBOLS.contains(&c)));
        assert!(pw.bytes().any(|c| LOWER.contains(&c)));
        assert!(pw.bytes().any(|c| UPPER.contains(&c)));
    }

    #[test]
    fn password_avoids_ambiguous() {
        let opts = PasswordOptions {
            avoid_ambiguous: true,
            ..Default::default()
        };
        for _ in 0..20 {
            let pw = password(&opts).unwrap();
            assert!(!pw.bytes().any(|c| AMBIGUOUS.contains(&c)));
        }
    }

    #[test]
    fn passphrase_word_count() {
        let pp = passphrase(5, '-').unwrap();
        assert_eq!(pp.split('-').count(), 5);
    }

    #[test]
    fn pin_digits_only() {
        let p = pin(6).unwrap();
        assert_eq!(p.len(), 6);
        assert!(p.bytes().all(|b| b.is_ascii_digit()));
    }
}
