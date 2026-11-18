//! Arbitrary release tables and stored files in an otherwise well-formed pack, so
//! table parsing and validation run past the SHA-256 checks.
//!
//! The input is split on NUL bytes: the release name, the seven tables in
//! `TABLES` order, then any number of stored files (kept under `blobs/<sha256>`).
#![no_main]

use glibc_provider_sources::{encode, PackEntry, Sources, TABLES};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut parts = data.split(|&b| b == 0);
    let release = String::from_utf8_lossy(parts.next().unwrap_or_default()).into_owned();
    let mut entries: Vec<PackEntry> = Vec::new();
    for table in TABLES {
        let text = parts.next().unwrap_or_default().to_vec();
        entries.push((format!("{release}/{table}"), text, None));
    }
    for blob in parts {
        entries.push((format!("blobs/{}", sha256_hex(blob)), blob.to_vec(), None));
    }
    let Ok(pack) = encode(&entries) else { return };
    if let Ok(sources) = Sources::parse(pack) {
        glibc_provider_fuzz::exercise(&sources);
    }
});

/// The blob's name; borrowed from a pack round trip so no hashing crate is needed.
fn sha256_hex(blob: &[u8]) -> String {
    let pack = encode(&[("x".to_string(), blob.to_vec(), None)]).expect("one entry");
    let text = String::from_utf8_lossy(&pack);
    let line = text.lines().nth(2).expect("index line");
    line.split('\t').nth(1).expect("digest").to_string()
}
