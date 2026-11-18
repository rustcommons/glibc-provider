//! Arbitrary bytes as an uncompressed pack: index lines, entry sizes, line deltas,
//! SHA-256 checks and per-release table validation.
#![no_main]

use glibc_provider_sources::Sources;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(sources) = Sources::parse(data.to_vec()) {
        glibc_provider_fuzz::exercise(&sources);
    }
});
