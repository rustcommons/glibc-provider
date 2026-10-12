# Notice

## The data

`data/blobs/` holds files copied, unmodified, from three origins. Each release's
`files.tsv` and `headers.tsv` record which origin each file has.

### GNU C Library (glibc)

- **What:** source files and the headers generated while building them, from the
  official GNU release archives `glibc-2.17.tar.xz` through `glibc-2.44.tar.xz`
  (https://ftp.gnu.org/gnu/glibc/). Each archive's SHA-256 is recorded in
  `data/X.Y/release.tsv`, and its detached signature was verified against the GNU
  keyring when the archive was pinned in linux-abi-tools.
- **Who:** Copyright (C) Free Software Foundation, Inc. and the glibc contributors.
- **License:** the GNU Lesser General Public License, version 2.1 or later
  (`LGPL-2.1-or-later`; [LICENSES/LGPL-2.1-or-later.txt](LICENSES/LGPL-2.1-or-later.txt)).
  Several startup files also carry glibc's additional permission to link their
  compiled form into any program.
- **Generated headers:** these come from glibc's own configure and make run on the
  archive, and are distributed under glibc's license.

### Linux kernel UAPI headers

- **What:** the builder's Linux user-space API headers (`linux/`, `asm/`,
  `asm-generic/`), as installed in each provider and as included by the startup
  sources.
- **Who:** the Linux kernel contributors.
- **License:** each file's own SPDX tag applies. Most are
  `GPL-2.0 WITH Linux-syscall-note`; some are dual-licensed with BSD-3-Clause or MIT.
  Older files without a tag are covered by the kernel's COPYING and its syscall note.
- **License texts:** [GPL-2.0-only](LICENSES/GPL-2.0-only.txt),
  [Linux-syscall-note](LICENSES/Linux-syscall-note.txt),
  [BSD-3-Clause](LICENSES/BSD-3-Clause.txt), and [MIT](LICENSES/MIT.txt), from the
  Linux 6.1 tag.

### Compiler headers

The builder compilers' own headers are recorded by path only and are not stored.

## The tables and the crate

The tables in `data/X.Y/` and the crate code in `src/` (when added) are distributed
under LGPL-2.1-or-later, the license of the data they describe.

## The crate

The `glibc-provider-sources` crate (`src/`) is licensed under LGPL-2.1-or-later. Its
`pack/sources.gz` holds all of `data/` unmodified (stored as line deltas between
versions and gzip-compressed, which the crate reverses exactly, checking every
SHA-256), so each file keeps its license above. The crate's SPDX expression names
them all: `LGPL-2.1-or-later AND GPL-2.0-only WITH Linux-syscall-note AND
GPL-2.0-or-later WITH Linux-syscall-note AND LGPL-2.1-only WITH Linux-syscall-note AND
LGPL-2.1-or-later WITH Linux-syscall-note AND BSD-3-Clause AND MIT`. Where a kernel
header is dual-licensed, the expression names its GPL choice. The crate's `sha256.rs`
is the collector's, below, relicensed by its authors.

## The collector

`collector/` is MIT licensed ([LICENSES/MIT.txt](LICENSES/MIT.txt)) and is not part
of any published crate. It contains no glibc code. Its `sha256.rs` is shared with
glibc-abilists' collector, also MIT.
