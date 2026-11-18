//! Arbitrary bytes as a gzip-compressed pack: gzip header, DEFLATE, CRC and size
//! checks, then the pack index, deltas and release tables.
#![no_main]

use glibc_provider_sources::Sources;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(sources) = Sources::from_gzip(data) {
        glibc_provider_fuzz::exercise(&sources);
    }
});
