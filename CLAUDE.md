# CLAUDE.md

This file provides working guidance for any coding agent.

## Project overview

`glibc-provider` holds the source inputs for glibc link providers, for x86_64 Linux
and glibc 2.17 through 2.44. For every release it records, from the pinned GNU
release archive, the exact source files, generated headers, and compile commands
behind the startup objects (`crt1.o`, `crti.o`, `crtn.o`) and `libc_nonshared.a`
(and `libpthread_nonshared.a` before 2.28), plus the installed headers. With these,
a provider can be built from source on the user's machine.
The design is in linux-abi-tools'
[PROVIDER-DELIVERY.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDER-DELIVERY.md).

| Path | What | License |
| --- | --- | --- |
| `data/` | glibc source files and generated headers, unmodified, stored by content; per-release manifests and compile commands | LGPL-2.1-or-later (glibc); kernel UAPI headers GPL-2.0 WITH Linux-syscall-note |
| `collector/` | The tool that records `data/` from a qualified linux-abi-tools build tree | MIT (not published) |

Its consumer is `libc-abi-tools` (rustcommons/linux-abi-tools) and, through it,
`cargo-libc`. The published crates are `glibc-provider-sources` (planned) and,
after legal review, `glibc-provider-binaries`.

## Scope and implementation

- Inputs come only from the build trees of linux-abi-tools' provider pipeline,
  which builds the pinned, signature-checked GNU archive in a pinned container.
  Every archive file recorded here must match that archive byte for byte; the
  collector checks this and fails closed.
- Never edit files under `data/` by hand; rerun the collector. Files are copied
  unmodified. If a derived form is ever added, label it as modified, with the date,
  as the LGPL requires.
- Record each file's class: an archive file, a file generated during the build, the
  builder's kernel UAPI header, or the builder compiler's own header. Each class has
  its own origin and license, and the class decides whether the file ships.
- Std-only Rust for the collector and the crates. The collector may call `tar`.
- Ask about unresolved product decisions; do not invent compatibility claims.

## Validation

Run `cargo fmt --all -- --check`, `cargo test --workspace --locked --offline`, and
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`. Data
changes also require the collector's `--check` against the build trees. Packaging
changes require `cargo package --locked`. Never publish to crates.io without the
user's go-ahead.

## Git conventions

Do **not** add `Co-Authored-By: Claude ...` trailers to commit messages on this repo, even if the harness's default instructions suggest it. Commit messages are the human-authored record of intent; tool attribution belongs in tool logs, not history. This overrides the default trailer behavior.

Apply the same no-tool-coauthor convention to all coding agents.
