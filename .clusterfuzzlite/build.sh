#!/bin/bash -eu
# Builds the fuzz targets in fuzz/ for ClusterFuzzLite (docs/FUZZING.md).
cd "$SRC/glibc-provider"
# The crate's MSRV stays 1.90; instrumentation needs this frozen nightly.
rustc --version | grep -F '1.91.0-nightly'
# cargo-fuzz has no --locked flag: fetch locked, build offline, then prove the
# lockfile is unchanged.
cargo fetch --locked --manifest-path fuzz/Cargo.toml
sha256sum fuzz/Cargo.lock > /tmp/fuzz-lock.sha256
CARGO_NET_OFFLINE=true cargo fuzz build -O --fuzz-dir fuzz
sha256sum --check /tmp/fuzz-lock.sha256
for target in gzip_pack pack tables delta; do
    cp "fuzz/target/x86_64-unknown-linux-gnu/release/$target" "$OUT/"
    cp "fuzz/$target.options" "$OUT/"
    (cd "fuzz/corpus/$target" && zip -q "$OUT/${target}_seed_corpus.zip" ./*)
done
