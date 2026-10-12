# Roadmap

The overall plan for provider delivery, and the work this repository is part of, is in
linux-abi-tools' [ROADMAP.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/ROADMAP.md#provider-delivery).

## Rust API documentation

Added 2026-10-11. The planned crates `glibc-provider-sources` and
`glibc-provider-binaries` must have complete Rust API documentation from their first
release. The collector and replay tool are not published.

- [ ] Write each crate's overview: what it contains, the licenses of its data, how
  libc-abi-tools uses it, and a runnable example, kept as a doc test.
- [ ] Document every public item from the start, with `#![warn(missing_docs)]`
  (CI denies warnings).
- [ ] Add `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS="-D warnings"` to CI, so
  broken intra-doc links and malformed docs fail the build.
- [ ] Add `documentation` (the docs.rs URL) and `[package.metadata.docs.rs]` (target
  `x86_64-unknown-linux-gnu`, all features documented) to each published Cargo.toml,
  and a docs.rs badge to each README.
- [ ] After each release, confirm the docs.rs build succeeded and the pages read
  correctly.
