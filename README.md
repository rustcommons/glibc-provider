# glibc-provider

Source inputs for building glibc link providers for **x86_64 Linux**.

[![crates.io](https://img.shields.io/crates/v/glibc-provider-sources)](https://crates.io/crates/glibc-provider-sources)
[![docs.rs](https://img.shields.io/docsrs/glibc-provider-sources)](https://docs.rs/glibc-provider-sources)

This repository publishes the `glibc-provider-sources` Rust library. It records
source files, generated and installed headers, compile commands, and object/library
metadata for glibc 2.17–2.44. linux-abi-tools uses these inputs to build link
providers; [cargo-libc](https://github.com/rustcommons/cargo-libc) coordinates their
use when building applications for an older glibc. Application developers should
start with cargo-libc.

## Use the library

Requires Rust 1.90 or newer. Add this dependency to your project's `Cargo.toml`:

```toml
[dependencies]
glibc-provider-sources = "0.1"
```

Put this example in `src/main.rs` and run `cargo run`:

```rust
fn main() -> Result<(), String> {
    let sources = glibc_provider_sources::Sources::bundled()?;
    let release = sources.release("2.28")?;
    for object in release.objects()? {
        println!("{}: {}", object.object, object.command);
    }
    Ok(())
}
```

The example reads bundled data and checks its SHA-256 hashes without writing
files. The std-only library embeds a roughly 4.7 MB pack that expands to about
72 MiB; reading and extracting it need no GCC, external decompressor, or container.
Extraction writes inputs, not compiled provider objects. Rebuilding those objects
requires GCC and binutils on x86_64 Linux; see the tested configurations and limits
in the [replay guide](https://github.com/rustcommons/glibc-provider/blob/main/docs/REPLAY.md).

## Guides

- [Reading and extracting data](https://github.com/rustcommons/glibc-provider/blob/main/docs/DATA.md#using-the-library), plus the [API reference](https://docs.rs/glibc-provider-sources).
- [Rebuilding and comparing objects](https://github.com/rustcommons/glibc-provider/blob/main/docs/REPLAY.md).
- [Collecting, regenerating, and validating data](https://github.com/rustcommons/glibc-provider/blob/main/docs/MAINTAINING.md).
- [Release plan](https://github.com/rustcommons/glibc-provider/blob/main/docs/RELEASE-PLAN.md) and [roadmap](https://github.com/rustcommons/glibc-provider/blob/main/docs/ROADMAP.md).

## Provenance and licenses

Inputs come from linux-abi-tools' qualified provider build trees, built from
pinned, signature-checked GNU archives. The collector verifies archive hashes,
checks recorded archive files against a fresh extraction, and matches target
objects against the installed build. Generated headers and kernel UAPI headers
retain their recorded origins. No glibc binary objects are shipped in this crate.

The crate code and glibc data are LGPL-2.1-or-later; Linux UAPI headers retain
their individual licenses, generally GPL-2.0 WITH Linux-syscall-note. The
unpublished collector is MIT. See [NOTICE.md](https://github.com/rustcommons/glibc-provider/blob/main/NOTICE.md)
for the full inventory and [DATA.md](https://github.com/rustcommons/glibc-provider/blob/main/docs/DATA.md)
for provenance classes and the pack format.
