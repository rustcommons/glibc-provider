//! Line deltas round-trip: a file stored as a delta of another must decode to
//! exactly its own bytes. The input is split at the first 0xff byte into the base
//! and the new file.
#![no_main]

use glibc_provider_sources::{encode, PackEntry, Sources};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let split = data.iter().position(|&b| b == 0xff).unwrap_or(data.len());
    let base = data[..split].to_vec();
    let new = data.get(split + 1..).unwrap_or_default().to_vec();
    let entries: Vec<PackEntry> = vec![
        ("a/base".to_string(), base.clone(), None),
        ("a/new".to_string(), new.clone(), Some("a/base".to_string())),
    ];
    let pack = encode(&entries).expect("valid entries encode");
    let sources = Sources::parse(pack).expect("an encoded pack parses");
    let read: Vec<_> = sources.entries().collect();
    assert_eq!(
        read,
        [("a/base", base.as_slice()), ("a/new", new.as_slice())]
    );
});
