//! `2pw` — CLI client for 2Password vaults.
//!
//! Secret Key resolution order (spec §4.1, §7):
//!   1. `--secret-key` / `TWOPASSWORD_SECRET_KEY`
//!   2. device file `~/.2password/device.json` (mode 0600; stands in for the
//!      platform keystore used by real clients)
//! Password: `TWOPASSWORD_PASSWORD` env or interactive prompt.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use twopassword_core::crypto::kdf::KdfParams;
use twopassword_core::crypto::SecretKey;
use twopassword_core::generate::{self, PasswordOptions};
use twopassword_core::vault::{ItemFile, ItemPayload, UnlockedVault};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "2pw", about = "2Password CLI client", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new vault directory
    Init {
        dir: PathBuf,
        /// Account id (e.g. your email). Stored unencrypted in profile.json.
        #[arg(long)]
        user: String,
    },
    /// Add an item to the vault
    Add {
        dir: PathBuf,
        #[arg(long)]
        title: String,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Generate a 20-char password instead of supplying one
        #[arg(long)]
        generate: bool,
    },
    /// Show an item by uuid or title
    Get {
        dir: PathBuf,
        /// uuid, exact title, or title substring
        query: String,
        /// Print the password in clear (otherwise masked)
        #[arg(long)]
        show_password: bool,
    },
    /// List item titles
    List { dir: PathBuf },
    /// Tombstone an item (deletion propagates via sync)
    Rm { dir: PathBuf, uuid: Uuid },
    /// Generate a password/passphrase/pin without touching the vault
    Gen {
        /// Random password of N chars
        #[arg(long)]
        length: Option<usize>,
        /// Passphrase of N words
        #[arg(long)]
        words: Option<usize>,
        /// PIN of N digits
        #[arg(long)]
        pin: Option<usize>,
        #[arg(long)]
        no_symbols: bool,
    },
}

fn device_file() -> Result<PathBuf> {
    let home = std::env::home_dir().context("cannot locate home directory")?;
    Ok(home.join(".2password/device.json"))
}

fn canonical(dir: &Path) -> Result<String> {
    Ok(dir.canonicalize()?.to_string_lossy().into_owned())
}

fn load_secret_key(dir: &Path) -> Result<SecretKey> {
    if let Ok(s) = std::env::var("TWOPASSWORD_SECRET_KEY") {
        return SecretKey::parse(&s).context("invalid TWOPASSWORD_SECRET_KEY");
    }
    let path = device_file()?;
    let map: BTreeMap<String, String> = path
        .exists()
        .then(|| fs::read_to_string(&path))
        .transpose()?
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    let key = map
        .get(&canonical(dir)?)
        .context("secret key not in device file; set TWOPASSWORD_SECRET_KEY")?;
    SecretKey::parse(key).context("invalid secret key in device file")
}

fn save_secret_key(dir: &Path, sk: &SecretKey) -> Result<()> {
    let path = device_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut map: BTreeMap<String, String> = path
        .exists()
        .then(|| fs::read_to_string(&path))
        .transpose()?
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    map.insert(canonical(dir)?, sk.to_display());
    fs::write(&path, serde_json::to_string_pretty(&map)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn password() -> Result<String> {
    if let Ok(p) = std::env::var("TWOPASSWORD_PASSWORD") {
        return Ok(p);
    }
    Ok(rpassword::prompt_password("Account password: ")?)
}

fn unlock(dir: &Path) -> Result<UnlockedVault> {
    let sk = load_secret_key(dir)?;
    let pw = password()?;
    UnlockedVault::unlock(dir, &pw, &sk).context("unlock failed (wrong password or secret key?)")
}

fn find_item(vault: &UnlockedVault, query: &str) -> Result<(ItemFile, ItemPayload)> {
    if let Ok(uuid) = Uuid::parse_str(query) {
        let file = vault.load_file(&uuid)?;
        let payload = vault.decrypt(&file)?;
        return Ok((file, payload));
    }
    let mut matches = Vec::new();
    for file in vault.item_files()? {
        if file.tombstone {
            continue;
        }
        let payload = vault.decrypt(&file)?;
        if payload.title.eq_ignore_ascii_case(query)
            || payload.title.to_lowercase().contains(&query.to_lowercase())
        {
            matches.push((file, payload));
        }
    }
    match matches.len() {
        0 => bail!("no item matching '{query}'"),
        1 => Ok(matches.pop().unwrap()),
        _ => {
            eprintln!("ambiguous query, candidates:");
            for (f, p) in &matches {
                eprintln!("  {}  {}", f.uuid, p.title);
            }
            bail!("refine the query")
        }
    }
}

fn print_item(file: &ItemFile, p: &ItemPayload, show_password: bool) {
    println!("uuid:      {}", file.uuid);
    println!("title:     {}", p.title);
    if let Some(u) = &p.username {
        println!("username:  {u}");
    }
    if let Some(u) = &p.url {
        println!("url:       {u}");
    }
    if let Some(pw) = &p.password {
        let shown = if show_password {
            pw.clone()
        } else {
            "•".repeat(8)
        };
        println!("password:  {shown}");
    }
    if let Some(n) = &p.notes {
        println!("notes:     {n}");
    }
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Init { dir, user } => {
            let pw = password()?;
            let sk = UnlockedVault::create(&dir, &user, KdfParams::default(), &pw)?;
            save_secret_key(&dir, &sk)?;
            println!("vault created at {}", dir.display());
            println!("secret key: {sk}");
            println!("(stored in {}; back it up in your emergency kit)", device_file()?.display());
        }
        Cmd::Add { dir, title, username, password: pw, url, notes, generate } => {
            let vault = unlock(&dir)?;
            let password = match (pw, generate) {
                (Some(p), _) => Some(p),
                (None, true) => {
                    let p = generate::password(&PasswordOptions::default())?;
                    println!("generated password: {p}");
                    Some(p)
                }
                (None, false) => None,
            };
            let id = vault.add(&ItemPayload { title, username, password, url, notes, ..Default::default() })?;
            println!("added {id}");
        }
        Cmd::Get { dir, query, show_password } => {
            let vault = unlock(&dir)?;
            let (file, payload) = find_item(&vault, &query)?;
            print_item(&file, &payload, show_password);
        }
        Cmd::List { dir } => {
            let vault = unlock(&dir)?;
            for file in vault.item_files()? {
                if file.tombstone {
                    continue;
                }
                let p = vault.decrypt(&file)?;
                println!("{}  {}  {}", file.uuid, p.title, p.username.unwrap_or_default());
            }
        }
        Cmd::Rm { dir, uuid } => {
            let vault = unlock(&dir)?;
            vault.tombstone(&uuid)?;
            println!("tombstoned {uuid}");
        }
        Cmd::Gen { length, words, pin, no_symbols } => {
            if let Some(n) = words {
                println!("{}", generate::passphrase(n, '-')?);
            } else if let Some(n) = pin {
                println!("{}", generate::pin(n)?);
            } else {
                let opts = PasswordOptions {
                    length: length.unwrap_or(20),
                    symbols: !no_symbols,
                    ..Default::default()
                };
                println!("{}", generate::password(&opts)?);
            }
        }
    }
    Ok(())
}
