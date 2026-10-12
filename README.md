# glibc-provider

**The exact glibc sources behind a link provider, for every release 2.17–2.44.**

A glibc *link provider* lets Rust (through
[cargo-libc](https://github.com/rustcommons/cargo-libc)) build for an older glibc
than the host has. Besides link stubs generated from glibc's ABI lists, a provider
needs real pieces of that glibc release: its startup objects (`crt1.o`, `crti.o`,
`crtn.o`), `libc_nonshared.a`, and its headers.

This repository records, for each release, the exact source files those pieces
were built from, the headers glibc's build generated for them, the compile
commands, and the installed headers. With them, a provider can be built from
source on a user's machine, with no container and no glibc binaries from us. The
design is in linux-abi-tools'
[PROVIDER-DELIVERY.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDER-DELIVERY.md).

**Status:** data collected and verified for all 28 releases. Rebuilding with the
host's GCC reproduces every release's objects ([docs/REPLAY.md](docs/REPLAY.md)); Clang
cannot for 2.17–2.42, because it ignores attributes glibc's sources apply after a
definition and would turn hidden symbols into exported ones. A local build therefore
uses the host's GCC, as glibc itself requires. The crate `glibc-provider-sources` is
built and qualified (below) and is not yet published to crates.io.

## The crate

`glibc-provider-sources` compiles all of `data/` in as one pack of 3.7 MB, which
decompresses to about 66 MiB:

```rust
let sources = glibc_provider_sources::Sources::bundled()?; // checks every SHA-256
let release = sources.release("2.28")?;
release.write_sources("/tmp/glibc-2.28-inputs".as_ref())?; // src/, build/, uapi/
release.write_headers("/tmp/glibc-2.28-include".as_ref())?; // a usr/include
for object in release.objects()? {
    println!("{} in {}: {}", object.object, object.directory, object.command);
}
```

It has no dependencies; the gzip decoder and SHA-256 are its own, std-only. The pack
format, and how it stays small, are in [docs/DATA.md](docs/DATA.md#the-crates-pack).
`glibc-provider-replay` builds every release from the crate, and CI checks that the
pack equals `data/` exactly.

## Where the data comes from

Only from the build trees of linux-abi-tools' provider pipeline. That pipeline builds
each release from the official, pinned, signature-checked GNU archive in a pinned
container, then audits and tests the result (84 hosted jobs). The collector:

- checks the archive's pinned SHA-256 and compares every recorded source file with
  a fresh extraction of it;
- checks every target against the installed copy, and matches archive members to
  objects by their bytes;
- records every input of every compiled object from glibc's own dependency files.

See [docs/DATA.md](docs/DATA.md) for the format, and [docs/ROADMAP.md](docs/ROADMAP.md)
for planned work.

## Regenerating

On Linux, with a linux-abi-tools work tree for the release (`examples/glibc-X.Y/run.sh`):

```sh
cargo run --release -p glibc-provider-collector -- \
  --work /tmp/glibc-2.28-work --release 2.28 --archive-sha256 <pinned SHA-256>
cargo run --release -p glibc-provider-collector -- \
  --work /tmp/glibc-2.28-work --release 2.28 --archive-sha256 <pinned SHA-256> --check
```

The pinned SHA-256 is in linux-abi-tools' `examples/glibc-X.Y/INPUTS.toml`.

## Licenses

The data is glibc's (LGPL-2.1-or-later) and, for the UAPI headers, the Linux
kernel's (generally GPL-2.0 WITH Linux-syscall-note). The collector is MIT. See
[NOTICE.md](NOTICE.md).
