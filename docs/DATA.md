# Data format

`data/` holds, for each glibc release 2.17 through 2.44, everything needed to
rebuild that release's startup objects and nonshared archives from source, plus
its installed headers. All of it is recorded by the collector from a qualified
build tree of linux-abi-tools' provider pipeline; nothing is edited by hand.

## Layout

| Path | Contents |
| --- | --- |
| `data/blobs/XX/<sha256>` | Every recorded file, stored once by its SHA-256 (`XX` is the hash's first two characters). Identical files shared by several releases are stored once. |
| `data/X.Y/release.tsv` | The release, the GNU archive and its pinned SHA-256, the collector version, and the SHA-256 of each target the container build produced |
| `data/X.Y/objects.tsv` | Each object behind each target, with its build command |
| `data/X.Y/inputs.tsv` | Each compiled object's inputs, from its dependency file |
| `data/X.Y/files.tsv` | The distinct inputs of the release, with class, SHA-256 and size |
| `data/X.Y/headers.tsv` | The installed headers (`usr/include`) of the release |
| `data/X.Y/interfaces.tsv` | The interface of each container-built object, the reference a replay is compared with ([REPLAY.md](REPLAY.md)) |

All tables are tab-separated with a header row, sorted, and written byte for byte
reproducibly (`--check` regenerates and compares).

## Targets

`crt1.o`, `crti.o`, `crtn.o`, `libc_nonshared.a`, and, before glibc 2.28,
`libpthread_nonshared.a`. Each is checked to equal the copy installed by the
container build, and its SHA-256 is recorded as `target.<name>`.

## `objects.tsv`

| Column | Meaning |
| --- | --- |
| `target` | The target the object belongs to |
| `member` | For an archive, the member's position (0-based); otherwise 0 |
| `object` | The object's path in the build tree, for example `csu/start.o` |
| `kind` | `compile` (built from one source file) or `relocatable` (a `-r` link of other objects, as `crt1.o` is) |
| `directory` | The glibc source subdirectory the command runs in (glibc compiles each object in the source subdirectory named like its build subdirectory) |
| `command` | The exact command from the build log; `@SRC@` stands for the glibc source tree and `@BUILD@` for the build tree |

Archive members are matched to build-tree objects by their exact bytes, so the
recorded objects are provably the ones in the archive.

## Input classes (`inputs.tsv`, `files.tsv`)

| Class | Path is relative to | Stored | Origin and license |
| --- | --- | --- | --- |
| `archive` | The top of the glibc source tree | Yes | The pinned GNU archive; checked byte for byte against a fresh extraction. glibc's license, LGPL-2.1-or-later. |
| `generated` | The build tree | Yes | Produced by glibc's configure and make from the archive (for example `config.h`, `libc-modules.h`, `tcb-offsets.h`); glibc's license. |
| `uapi` | `/usr/include` | Yes | The builder's Linux UAPI headers, as installed in the provider. The kernel's license, generally GPL-2.0 WITH Linux-syscall-note; each file's own SPDX tag applies. |
| `compiler` | Absolute path in the builder | No (path only) | The builder compiler's own headers (`stddef.h`, `stdarg.h`, `limits.h`, `cpuid.h`, ...). A local build uses its own compiler's equivalents. |
| `builder` | `builder-defaults.txt` | Yes | The builder compiler's predefined macros (`gcc -dM -E`). `release.tsv` derives `builder.gcc`, `builder.pie`, `builder.stack_protector`, `builder.cet` and `builder.fortify` from them: defaults that recorded commands leave implicit. |

## `interfaces.tsv`

Recorded from the container build tree by `glibc-provider-replay --release X.Y
--record WORK`. One row per fact, in the comparison's terms
([REPLAY.md](REPLAY.md#how-objects-are-compared)):

| `kind` | `value` |
| --- | --- |
| `defined` | A defined global or weak symbol: name, type, binding, visibility, section |
| `undefined` | An undefined global or weak symbol: name, binding |
| `section` | A non-empty allocated section: name, flags |
| `reference` | A reference: relocation section, form, symbol |
| `bytes` | SHA-256 of the allocated sections' names and contents |

It describes the objects; it contains no code or data from them.

## Measured totals (2026-10-11)

| | Count |
| --- | --- |
| Releases | 28 (2.17 through 2.44) |
| Objects per release | 22–23 before 2.33, 11–12 from 2.33 (the `stat` wrappers left `libc_nonshared.a`) |
| Distinct inputs per release | 175 (2.17) to 351 (2.44) |
| Installed headers per release | 1083 (2.17) to 1348 (2.44) |
| Stored files (deduplicated) | 9,631, 76 MiB uncompressed, 2.4 MiB with xz |

The archive inputs carry glibc's LGPL notice (2,939 distinct files) or no notice of
their own (313 small headers such as wrapper `bits/` headers), which glibc's
`COPYING.LIB` covers. No archive input is under another license.

## The crate's pack

`pack/sources.gz` is all of `data/` in one file, written by `glibc-provider-pack` and
compiled into the `glibc-provider-sources` crate.

- **Contents:** every release's six tables (`2.28/release.tsv` …) and every stored
  file they name (`blobs/<sha256>`), each once. Compiler headers, recorded by path
  only, are not stored.
- **Index:** the first line is `glibc-provider-pack 1`, the second the entry count,
  then one line per entry: name, SHA-256 and size of the contents, size of what is
  stored, and the base entry (or `-`). The stored bytes follow in index order.
- **Deltas:** an entry with a base is stored as a line delta against that earlier
  entry: `c<N>` copies N base lines, `s<N>` skips N, and `a<N>` inserts the N bytes
  that follow. A table's base is the same table of the previous release; a file's,
  the previous version of the same path. A delta is used only where it is smaller
  (the encoder gives up beyond 3,000 differing lines and stores the file whole).
- **Compression:** the index and stored bytes are compressed with `gzip -9 -n`.
- **Checks:** the crate rebuilds every entry and requires its SHA-256 and size; then
  every file a release's tables name must be present with that hash and size, and
  every path must be relative with no `..`. `glibc-provider-pack --check` requires the
  pack to equal `data/` exactly.

| | Bytes |
| --- | ---: |
| Entries (9,802: 168 tables, 9,634 files) | 68,678,042 |
| Stored, with deltas | 15,943,011 |
| `pack/sources.gz` | 3,717,984 |
| Without deltas, for comparison | 8,670,324 |

Without deltas gzip cannot reach earlier versions of a file (its window is 32 KiB),
which would leave only a few releases of room under crates.io's 10 MB limit.

