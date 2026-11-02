# Changelog

Noteworthy changes to **glibc-provider-sources**, the library published from
`rustcommons/glibc-provider`. Dates below are the original crates.io publication
dates. Entries and versioned notes were backfilled on 2026-11-02 from source
tags, registry records and the linked validation evidence; they were not present
at the original source tags.

See the [versioned notes](https://github.com/rustcommons/glibc-provider/tree/main/docs/releases),
[GitHub Releases](https://github.com/rustcommons/glibc-provider/releases), and
[release policy](https://github.com/rustcommons/glibc-provider/blob/main/docs/RELEASE-PLAN.md#release-documentation-policy).

## [Unreleased]

### Documentation

- Added an architecture/data-flow diagram to the maintenance guide, separating
  maintainer recording, the published source-only library and local object builds.
- Added this changelog, versioned release notes and release-documentation policy
  navigation. These repository edits do not replace either published archive.

## [0.1.1] - 2026-11-01

### Fixed

- **The packaged README now describes a published library and gives usable setup.**
  The 0.1.0 README said the crate was not published, used relative links to excluded
  guides, and showed collector commands with an invalid hash placeholder and no
  `--bin`. Version 0.1.1 provides Rust 1.90 prerequisites, a complete non-writing
  example and absolute guide links; regeneration instructions live in the
  maintained guide.
- **Documentation-only patch:** no exported API, behavior, source pack, recorded
  tables, accepted differences or license change. Existing downstream 0.1.0 locks
  need no update for provider behavior; resolve 0.1.1 to obtain its packaged README.

[Versioned notes](https://github.com/rustcommons/glibc-provider/blob/main/docs/releases/v0.1.1.md)
include the exact released commit and bounded validation evidence.

## [0.1.0] - 2026-10-13

### Added

- **First published source-input library for x86_64 glibc 2.17–2.44.** Read source
  files, generated/installed headers, compile commands, builder defaults and
  object/library metadata for startup objects and nonshared archives. Consumers
  can extract inputs to new directories and build providers locally.
- **Std-only Rust API and embedded pack:** no external Rust dependencies; the
  library decodes the roughly 4.7 MB source pack and verifies its hashes without
  invoking a compiler or external decompressor. Rust 1.90 is the minimum.
- Included public definitions for eight shared libraries so downstream source
  providers can generate link stubs without shipping glibc binaries, and reviewed
  object-comparison differences. Maintainer collection/replay tools are unpublished.

### Known documentation defects

- The immutable 0.1.0 packaged README has the status, guide-link and collector
  command defects described under 0.1.1. Use the corrected maintained guides or
  0.1.1 README; this backfill does not rewrite 0.1.0.

[Versioned notes](https://github.com/rustcommons/glibc-provider/blob/main/docs/releases/v0.1.0.md)
separate the historical source and validation record from current guides.

[Unreleased]: https://github.com/rustcommons/glibc-provider/compare/v0.1.1...main
[0.1.1]: https://github.com/rustcommons/glibc-provider/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/rustcommons/glibc-provider/tree/v0.1.0
