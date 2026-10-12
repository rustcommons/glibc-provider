# Roadmap

The overall plan for provider delivery, and the work this repository is part of, is in
linux-abi-tools' [ROADMAP.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/ROADMAP.md#provider-delivery).

## Data crates

- [x] Build `glibc-provider-sources` (2026-10-12): all 28 releases in one 3.7 MB pack,
  std-only decoding, checked by `glibc-provider-pack --check` and used by the replay.
- [x] Add each release's library definitions (`libraries.tsv`), so `source` mode
  generates link stubs without glibc binaries, and the reviewed differences
  (`ACCEPTED_DIFFERENCES`) (2026-10-12). The pack is now 4.7 MB.
- [ ] Publish it to crates.io together with the other crates at the release step
  (user decision 2026-10-12).
- [ ] `glibc-provider-binaries` (the `prebuilt` mode's CI-built objects), after the
  legal review: storing glibc binaries in this public repository already distributes
  them, so neither the crate's data nor its objects are added before that review.

## Rust API documentation

Added 2026-10-11. The planned crates `glibc-provider-sources` and
`glibc-provider-binaries` must have complete Rust API documentation from their first
release. The collector and replay tool are not published.

- [x] Write each crate's overview: what it contains, the licenses of its data, how
  libc-abi-tools uses it, and a runnable example, kept as a doc test
  (`glibc-provider-sources`, 2026-10-12).
- [x] Document every public item from the start, with `#![warn(missing_docs)]`
  (CI denies warnings) (`glibc-provider-sources`).
- [x] Add `cargo doc --no-deps` with `RUSTDOCFLAGS="-D warnings"` to CI, so
  broken intra-doc links and malformed docs fail the build.
- [ ] Add `documentation` (the docs.rs URL) and `[package.metadata.docs.rs]` (target
  `x86_64-unknown-linux-gnu`, all features documented) to each published Cargo.toml,
  and a docs.rs badge to each README. Done for `glibc-provider-sources` except the
  badge, which waits for its first release.
- [ ] After each release, confirm the docs.rs build succeeded and the pages read
  correctly.
