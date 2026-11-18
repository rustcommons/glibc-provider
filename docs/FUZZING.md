# Fuzzing

The [fuzz workspace](../fuzz/Cargo.toml) feeds arbitrary bytes to the crate's
public pack loaders and asserts they return `Ok` or `Err` without panicking. It is
a separate, unpublished workspace with its own lockfile: `libfuzzer-sys` (and the
`arbitrary` crate it pulls in) never enters the published crate's dependency
graph, and the crate's `include` list keeps `fuzz/` out of its package. The owner
approved these fuzz-only dependencies on 2026-11-17.

| Target | Entry point | What it reaches | Seeds |
| --- | --- | --- | --- |
| `gzip_pack` | `Sources::from_gzip` | gzip header, DEFLATE (stored, fixed and dynamic blocks), CRC-32/size trailer, then the pack | the inflate tests' gzip data and a gzipped one-release pack |
| `pack` | `Sources::parse` | pack index, entry sizes, line deltas, SHA-256 checks, per-release table validation | the unit tests' one-release pack and delta pack |
| `tables` | `encode` then `Sources::parse` | arbitrary `release.tsv`, `objects.tsv`, `files.tsv`, `headers.tsv` and stored files in a correctly hashed pack, so table parsing and validation run past the SHA-256 checks | the unit tests' release tables |
| `delta` | `encode` then `Sources::parse` | line-delta encoding and decoding must round-trip any pair of files | the unit tests' appended and edited files |

Every target that loads a pack then reads all its releases, tables and files
through the public API (`fuzz/src/lib.rs`). The seeds in `fuzz/corpus/` are
synthetic copies of unit-test fixtures, not glibc data. Inputs are capped at
64 KiB (`fuzz/*.options`). Targets touch no network or files.

The collector's parsers (ar archives, depfiles, build logs) read only the
maintainer's own qualified build trees and are not fuzzed.

## Run locally

Ordinary tests use the stable toolchain. Fuzzing needs the nightly the CI builder
ships, cargo-fuzz and a C++ compiler:

```sh
rustup toolchain install nightly-2025-09-05 --profile minimal --component rust-src
cargo install cargo-fuzz --version =0.13.1 --locked
cargo +nightly-2025-09-05 fetch --locked --manifest-path fuzz/Cargo.toml
CARGO_NET_OFFLINE=true cargo +nightly-2025-09-05 fuzz run --fuzz-dir fuzz pack fuzz/corpus/pack -- -max_total_time=60
```

Replace `pack` with any target above. cargo-fuzz has no `--locked` flag, so fetch
with `--locked` first and run offline; `fuzz/Cargo.lock` must stay unchanged. Pass
a scratch directory before `fuzz/corpus/<target>` to keep new inputs out of Git.

A crash is a bug. Reproduce it with
`cargo +nightly-2025-09-05 fuzz run --fuzz-dir fuzz <target> <crash-file>`, add a
deterministic regression test next to the code, then fix it. Never catch panics
or ignore crashes to make fuzzing pass. The first run found that a corrupt delta
could panic `Sources::parse` (fixed, with `delta::tests::rejects_bad_deltas`).

## Continuous fuzzing

[ClusterFuzzLite](https://google.github.io/clusterfuzzlite/) builds the targets
with AddressSanitizer in the digest-pinned OSS-Fuzz Rust builder
([Dockerfile](../.clusterfuzzlite/Dockerfile), [build.sh](../.clusterfuzzlite/build.sh)),
which ships nightly-2025-09-05 and cargo-fuzz; the build downloads no installer
and checks that `fuzz/Cargo.lock` is unchanged.

- [PR fuzzing](../.github/workflows/cflite_pr.yml): code-change mode, 300 seconds
  in total, on PRs that touch `src/`, `fuzz/`, the fuzz configuration or the root
  manifest/lockfile. New commits cancel the PR's older run.
- [Weekly batch](../.github/workflows/cflite_batch.yml): 1800 seconds in total.

Both skip Dependabot PRs, can be started by hand, and upload crash SARIF to code
scanning (category `clusterfuzzlite`) when a crash is found; fork PRs get no
upload permission. The ClusterFuzzLite actions are pinned by commit SHA; their own
runner images are chosen upstream. No corpus storage repository is configured, so
batch corpora live only as run artifacts; reviewed seeds live in Git.
