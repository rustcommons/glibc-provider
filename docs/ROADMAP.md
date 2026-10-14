# Roadmap

Open work only. The overall plan for provider delivery is in linux-abi-tools'
[ROADMAP.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/ROADMAP.md#provider-delivery).

## Data crates

- [ ] `glibc-provider-binaries` (the `prebuilt` mode's CI-built objects), after the
  legal review: storing glibc binaries in this public repository already distributes
  them, so neither the crate's data nor its objects are added before that review.
  It needs complete Rust API documentation from its first release, as
  `glibc-provider-sources` has: an overview with a doc-test example, every public item
  documented under `#![warn(missing_docs)]`, and docs.rs metadata.

## Rust API documentation

- [ ] Add a docs.rs badge to the README.
- [ ] After each release, confirm the docs.rs build succeeded and the pages read
  correctly.
