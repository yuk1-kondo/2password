//! Chrome/Firefox native-messaging host bridge (spec §8.3).
//!
//! Framing: stdin/stdout carry 4-byte little-endian length + JSON frames,
//! forwarded line-delimited to the app over a Unix domain socket at
//! `~/Library/Application Support/2password/app.sock`. The app answers only
//! while unlocked — this binary holds no secrets and cannot open the vault.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn socket_path() -> Option<PathBuf> {
    std::env::home_dir().map(|h| {
        h.join("Library/Application Support/2password/app.sock")
    })
}

fn handle(msg: &[u8]) -> Vec<u8> {
    let Some(path) = socket_path() else {
        return br#"{"error":"no socket path"}"#.to_vec();
    };
    match UnixStream::connect(&path) {
        Err(_) => br#"{"error":"2Password is locked or not running"}"#.to_vec(),
        Ok(mut stream) => {
            if stream.write_all(msg).and_then(|_| stream.write_all(b"\n")).is_err() {
                return br#"{"error":"socket write failed"}"#.to_vec();
            }
            let mut line = String::new();
            match BufReader::new(stream).read_line(&mut line) {
                Ok(0) | Err(_) => br#"{"error":"no response from 2Password"}"#.to_vec(),
                Ok(_) => line.trim_end().as_bytes().to_vec(),
            }
        }
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    loop {
        let mut len = [0u8; 4];
        if stdin.read_exact(&mut len).is_err() {
            break; // extension closed the pipe
        }
        let n = u32::from_le_bytes(len) as usize;
        if n > 1_048_576 {
            break; // protocol sanity bound
        }
        let mut msg = vec![0u8; n];
        if stdin.read_exact(&mut msg).is_err() {
            break;
        }
        let resp = handle(&msg);
        let _ = stdout.write_all(&(resp.len() as u32).to_le_bytes());
        let _ = stdout.write_all(&resp);
        let _ = stdout.flush();
    }
}
