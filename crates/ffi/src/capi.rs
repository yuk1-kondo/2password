//! Plain C ABI for platforms without UniFFI (Windows C#/P-Invoke via
//! csbindgen). All item payloads cross the boundary as JSON strings so no
//! struct marshalling is needed; errors are reported through
//! `tp_last_error()` (thread-local, do not free).

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use twopassword_core::crypto::kdf::KdfParams;
use twopassword_core::crypto::twoskd::derive_auk;
use twopassword_core::crypto::{SecretKey, SymmetricKey};
use twopassword_core::generate::{self, PasswordOptions};
use twopassword_core::vault::{ItemPayload, UnlockedVault};

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("").unwrap());
}

fn set_err(e: impl std::fmt::Display) {
    let msg = e.to_string().replace('\0', " ");
    LAST_ERROR.with(|c| *c.borrow_mut() = CString::new(msg).unwrap());
}

fn clear_err() {
    LAST_ERROR.with(|c| *c.borrow_mut() = CString::new("").unwrap());
}

/// Thread-local error string (owned by the library — do not free).
#[no_mangle]
pub extern "C" fn tp_last_error() -> *const c_char {
    LAST_ERROR.with(|c| c.borrow().as_ptr())
}

unsafe fn cstr<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

fn out(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

/// Free a string returned by any `tp_*` function.
#[no_mangle]
pub unsafe extern "C" fn tp_string_free(p: *mut c_char) {
    if !p.is_null() {
        drop(CString::from_raw(p));
    }
}

/// Free a vault handle from `tp_vault_unlock*`.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_free(p: *mut UnlockedVault) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}

/// Generate a new Secret Key in display format (`P1-...`). Free the result.
#[no_mangle]
pub extern "C" fn tp_secret_key_generate() -> *mut c_char {
    out(SecretKey::generate().to_display())
}

/// Create a vault at `dir` for `user_id`, wrapping the keyset under an AUK
/// derived from `password` + `secret_key`. Returns 0/-1.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_init(
    dir: *const c_char,
    user_id: *const c_char,
    password: *const c_char,
    secret_key: *const c_char,
) -> i32 {
    clear_err();
    let (Some(d), Some(u), Some(pw), Some(sk)) =
        (cstr(dir), cstr(user_id), cstr(password), cstr(secret_key))
    else {
        set_err("null argument");
        return -1;
    };
    let sk = match SecretKey::parse(sk) {
        Ok(s) => s,
        Err(e) => {
            set_err(e);
            return -1;
        }
    };
    match UnlockedVault::create_with_key(Path::new(d), u, KdfParams::default(), pw, &sk) {
        Ok(_) => 0,
        Err(e) => {
            set_err(e);
            -1
        }
    }
}

fn unlock_result(v: twopassword_core::Result<UnlockedVault>) -> *mut UnlockedVault {
    match v {
        Ok(v) => Box::into_raw(Box::new(v)),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

/// Unlock a vault. Returns an opaque handle or NULL (see `tp_last_error`).
#[no_mangle]
pub unsafe extern "C" fn tp_vault_unlock(
    dir: *const c_char,
    password: *const c_char,
    secret_key: *const c_char,
) -> *mut UnlockedVault {
    clear_err();
    let (Some(d), Some(pw), Some(sk)) = (cstr(dir), cstr(password), cstr(secret_key)) else {
        set_err("null argument");
        return std::ptr::null_mut();
    };
    let sk = match SecretKey::parse(sk) {
        Ok(s) => s,
        Err(e) => {
            set_err(e);
            return std::ptr::null_mut();
        }
    };
    unlock_result(UnlockedVault::unlock(Path::new(d), pw, &sk))
}

/// Unlock using a stored AUK (platform biometrics path). Returns handle/NULL.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_unlock_auk(
    dir: *const c_char,
    auk_b64: *const c_char,
) -> *mut UnlockedVault {
    clear_err();
    let (Some(d), Some(a)) = (cstr(dir), cstr(auk_b64)) else {
        set_err("null argument");
        return std::ptr::null_mut();
    };
    let Ok(bytes) = data_encoding::BASE64.decode(a.as_bytes()) else {
        set_err("invalid base64");
        return std::ptr::null_mut();
    };
    let Ok(key): Result<[u8; 32], _> = bytes.try_into() else {
        set_err("invalid auk length");
        return std::ptr::null_mut();
    };
    unlock_result(UnlockedVault::unlock_with_auk(
        Path::new(d),
        &SymmetricKey::new(key),
    ))
}

/// Derive the AUK as base64 (for platform biometric storage). Free the result.
#[no_mangle]
pub unsafe extern "C" fn tp_derive_auk(
    dir: *const c_char,
    password: *const c_char,
    secret_key: *const c_char,
) -> *mut c_char {
    clear_err();
    let (Some(d), Some(pw), Some(sk)) = (cstr(dir), cstr(password), cstr(secret_key)) else {
        set_err("null argument");
        return std::ptr::null_mut();
    };
    let (Ok(profile), Ok(sk)) = (
        UnlockedVault::read_profile(Path::new(d)),
        SecretKey::parse(sk),
    ) else {
        set_err("invalid dir or secret key");
        return std::ptr::null_mut();
    };
    match derive_auk(pw.as_bytes(), &sk, &profile.account) {
        Ok(auk) => out(data_encoding::BASE64.encode(auk.as_bytes())),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

/// List items as a JSON array [{uuid,title,username,url,favorite}]. Free result.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_list(v: *mut UnlockedVault) -> *mut c_char {
    clear_err();
    let Some(v) = v.as_ref() else {
        set_err("null vault");
        return std::ptr::null_mut();
    };
    match v.item_files() {
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
        Ok(files) => {
            let mut arr = Vec::new();
            for f in files {
                if f.tombstone {
                    continue;
                }
                match v.decrypt(&f) {
                    Ok(p) => arr.push(serde_json::json!({
                        "uuid": f.uuid.to_string(),
                        "title": p.title,
                        "username": p.username,
                        "url": p.url,
                        "favorite": p.favorite,
                    })),
                    Err(e) => {
                        set_err(e);
                        return std::ptr::null_mut();
                    }
                }
            }
            out(serde_json::to_string(&arr).unwrap_or_else(|_| "[]".into()))
        }
    }
}

/// Get one item as JSON {uuid,title,username,password,url,notes,totp,favorite}.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_get(v: *mut UnlockedVault, uuid: *const c_char) -> *mut c_char {
    clear_err();
    let (Some(u), Some(v)) = (cstr(uuid), v.as_ref()) else {
        set_err("null argument");
        return std::ptr::null_mut();
    };
    let Ok(uu) = uuid::Uuid::parse_str(u) else {
        set_err("invalid uuid");
        return std::ptr::null_mut();
    };
    match v.load_file(&uu).and_then(|f| {
        if f.tombstone {
            Err(twopassword_core::Error::Malformed)
        } else {
            v.decrypt(&f)
        }
    }) {
        Ok(p) => out(serde_json::json!({
            "uuid": uu.to_string(),
            "title": p.title,
            "username": p.username,
            "password": p.password,
            "url": p.url,
            "notes": p.notes,
            "totp": p.totp_seed,
            "favorite": p.favorite,
        })
        .to_string()),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

fn write_item<F>(v: &UnlockedVault, item_json: *const c_char, f: F) -> i32
where
    F: FnOnce(&UnlockedVault, &ItemPayload) -> twopassword_core::Result<()>,
{
    let Some(j) = (unsafe { cstr(item_json) }) else {
        set_err("null json");
        return -1;
    };
    let Ok(p) = serde_json::from_str::<ItemPayload>(j) else {
        set_err("invalid json");
        return -1;
    };
    match f(v, &p) {
        Ok(()) => 0,
        Err(e) => {
            set_err(e);
            -1
        }
    }
}

/// Add an item from a JSON payload. Returns 0/-1.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_add(v: *mut UnlockedVault, item_json: *const c_char) -> i32 {
    clear_err();
    let Some(v) = v.as_ref() else {
        set_err("null vault");
        return -1;
    };
    write_item(v, item_json, |v, p| v.add(p).map(|_| ()))
}

/// Update an item from a JSON payload. Returns 0/-1.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_update(
    v: *mut UnlockedVault,
    uuid: *const c_char,
    item_json: *const c_char,
) -> i32 {
    clear_err();
    let (Some(u), Some(v)) = (cstr(uuid), v.as_ref()) else {
        set_err("null argument");
        return -1;
    };
    let Ok(uu) = uuid::Uuid::parse_str(u) else {
        set_err("invalid uuid");
        return -1;
    };
    write_item(v, item_json, |v, p| v.update(&uu, p))
}

/// Tombstone an item so deletion propagates (spec §6). Returns 0/-1.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_remove(v: *mut UnlockedVault, uuid: *const c_char) -> i32 {
    clear_err();
    let (Some(u), Some(v)) = (cstr(uuid), v.as_ref()) else {
        set_err("null argument");
        return -1;
    };
    let Ok(uu) = uuid::Uuid::parse_str(u) else {
        set_err("invalid uuid");
        return -1;
    };
    match v.tombstone(&uu) {
        Ok(_) => 0,
        Err(e) => {
            set_err(e);
            -1
        }
    }
}

/// Change the account password (re-wraps keyset only, spec §5). Returns 0/-1.
#[no_mangle]
pub unsafe extern "C" fn tp_vault_change_password(
    v: *mut UnlockedVault,
    new_password: *const c_char,
    secret_key: *const c_char,
) -> i32 {
    clear_err();
    let (Some(pw), Some(sk), Some(v)) = (cstr(new_password), cstr(secret_key), v.as_ref()) else {
        set_err("null argument");
        return -1;
    };
    let Ok(sk) = SecretKey::parse(sk) else {
        set_err("invalid secret key");
        return -1;
    };
    match v.change_password(pw, &sk) {
        Ok(_) => 0,
        Err(e) => {
            set_err(e);
            -1
        }
    }
}

/// Generate a random password. Free the result.
#[no_mangle]
pub unsafe extern "C" fn tp_gen_password(
    length: i32,
    digits: i32,
    symbols: i32,
    avoid_ambiguous: i32,
) -> *mut c_char {
    let opts = PasswordOptions {
        length: length.max(4) as usize,
        digits: digits != 0,
        symbols: symbols != 0,
        avoid_ambiguous: avoid_ambiguous != 0,
    };
    match generate::password(&opts) {
        Ok(s) => out(s),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

/// Generate an EFF passphrase. Free the result.
#[no_mangle]
pub unsafe extern "C" fn tp_gen_passphrase(
    word_count: i32,
    separator: *const c_char,
) -> *mut c_char {
    let sep = unsafe { cstr(separator) }
        .and_then(|s| s.chars().next())
        .unwrap_or('-');
    match generate::passphrase(word_count.max(3) as usize, sep) {
        Ok(s) => out(s),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

/// TOTP code at unix timestamp (RFC 6238). Free the result.
#[no_mangle]
pub unsafe extern "C" fn tp_totp_code(seed: *const c_char, timestamp_secs: i64) -> *mut c_char {
    let Some(s) = (unsafe { cstr(seed) }) else {
        set_err("null seed");
        return std::ptr::null_mut();
    };
    match twopassword_core::totp::code(s, timestamp_secs.max(0) as u64) {
        Ok(c) => out(c),
        Err(e) => {
            set_err(e);
            std::ptr::null_mut()
        }
    }
}

/// HIBP breach count via k-anonymity (spec §9). -1 = error.
#[no_mangle]
pub unsafe extern "C" fn tp_breach_count(password: *const c_char) -> i64 {
    let Some(pw) = (unsafe { cstr(password) }) else {
        set_err("null password");
        return -1;
    };
    match twopassword_core::hibp::breach_count(pw) {
        Ok(n) => n as i64,
        Err(e) => {
            set_err(e);
            -1
        }
    }
}
