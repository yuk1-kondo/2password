//! CSV import/export (spec §9). Plaintext exports carry a warning at the UI
//! layer; this module just moves data.

use crate::vault::ItemPayload;
use crate::{Error, Result};
use std::path::Path;

/// Import a CSV file. Recognizes 1Password export headers
/// (`Title,Url,Username,Password,OTPAuth,...,Notes`) and generic
/// `title,username,password,url,totp,notes` order (with or without header).
pub fn import_csv(path: &Path) -> Result<Vec<ItemPayload>> {
    let file = std::fs::File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false) // we detect headers manually
        .flexible(true)
        .from_reader(file);

    let mut records = rdr.records();
    let first = records.next().transpose().map_err(io)?.unwrap_or_default();
    let headers: Vec<String> = first.iter().map(|s| s.trim().to_lowercase()).collect();
    let is_header = headers.iter().any(|h| h.contains("title") || h.contains("password"));

    let default_cols = ["title", "username", "password", "url", "totp", "notes"]
        .map(String::from);
    let cols: &[String] = if is_header { &headers } else { &default_cols };

    let idx = |names: &[&str]| -> Option<usize> {
        names.iter().find_map(|n| cols.iter().position(|c| c.contains(n)))
    };
    let (i_title, i_user, i_pass, i_url, i_totp, i_notes, i_fav) = (
        idx(&["title", "name"]),
        idx(&["username", "user", "login"]),
        idx(&["password"]),
        idx(&["url", "website"]),
        idx(&["totp", "otpauth", "otp"]),
        idx(&["notes", "comment"]),
        idx(&["favorite"]),
    );

    let mut out = Vec::new();
    let iter: Box<dyn Iterator<Item = std::result::Result<csv::StringRecord, csv::Error>>> =
        if is_header {
            Box::new(records)
        } else {
            Box::new(std::iter::once(Ok(first)).chain(records))
        };

    for rec in iter {
        let rec = rec.map_err(io)?;
        let get = |i: Option<usize>| -> Option<String> {
            i.and_then(|i| rec.get(i))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let title = get(i_title).unwrap_or_default();
        if title.is_empty() {
            continue;
        }
        out.push(ItemPayload {
            title,
            username: get(i_user),
            password: get(i_pass),
            url: get(i_url),
            totp_seed: get(i_totp).and_then(|s| crate::totp::normalize_seed(&s).ok()),
            notes: get(i_notes),
            favorite: get(i_fav).is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        });
    }
    Ok(out)
}

/// Export all items to CSV. Plaintext — callers must warn the user (spec §9).
pub fn export_csv(path: &Path, items: &[ItemPayload]) -> Result<()> {
    let mut w = csv::Writer::from_path(path).map_err(io)?;
    w.write_record(["title", "username", "password", "url", "totp", "notes", "favorite"])
        .map_err(io)?;
    for it in items {
        w.write_record([
            it.title.as_str(),
            it.username.as_deref().unwrap_or(""),
            it.password.as_deref().unwrap_or(""),
            it.url.as_deref().unwrap_or(""),
            it.totp_seed.as_deref().unwrap_or(""),
            it.notes.as_deref().unwrap_or(""),
            if it.favorite { "1" } else { "" },
        ])
        .map_err(io)?;
    }
    w.flush().map_err(io)
}

fn io(e: impl std::error::Error) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}
