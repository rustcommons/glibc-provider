# Rebuilding the objects locally

`glibc-provider-replay` rebuilds a release's startup objects and nonshared
archives from the recorded inputs with a local compiler, and compares every
object with the container build. This is the qualification for linux-abi-tools'
`source` delivery mode ([PROVIDER-DELIVERY.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDER-DELIVERY.md)).

```sh
glibc-provider-replay --release 2.28 --compiler gcc --out /tmp/replay-2.28
qualification/replay-all.sh path/to/glibc-provider-replay gcc /tmp/replay
```

Each object is compared with the release's recorded `interfaces.tsv`
([DATA.md](DATA.md#interfacestsv)), so no container build is needed. With a build
tree at hand, `--compare /tmp/glibc-2.28-work` compares with its objects directly,
and `--record /tmp/glibc-2.28-work` (re)records `interfaces.tsv` from them.

## How a command is replayed

1. **Recreate the inputs** from the `glibc-provider-sources` crate, as a local
   provider build does: `Release::write_sources` writes the recorded archive files,
   generated headers and kernel UAPI headers, after the crate has checked every
   SHA-256. Regenerate the pack (`glibc-provider-pack`) after changing `data/`.
2. **Run each recorded command:**
   - `gcc` is replaced by the chosen compiler, run from the recorded source
     subdirectory, with dependency-file options dropped.
   - `-nostdinc` is added, with only the compiler's own built-in headers and the
     recorded UAPI headers on the system include path. No host header is visible.
   - The builder compiler's implicit defaults, recorded from its predefined macros
     (`builder.*` in `release.tsv`), are made explicit wherever the command does not
     set them: `-fPIE` or `-fno-pie`, and no stack protector, CET or fortification.
     The replaying compiler's own distribution defaults are switched off in the same
     way (`-fcf-protection=none`, `-fno-stack-clash-protection`, `-U_FORTIFY_SOURCE`,
     and, where the assembler accepts it, `-Wa,-mx86-used-note=no`: Fedora's GNU as
     otherwise adds an x86 "ISA used" `.note.gnu.property` to every object).
3. **Repeat the links and archives.** The `crt1.o` relocatable link is repeated
   (`-no-pie`, no build ID), and each archive is built in the recorded member order
   (`ar rcD`).

## How objects are compared

| Must match | Notes |
| --- | --- |
| Defined global and weak symbols: name, type, binding, visibility, section | |
| Undefined global and weak symbols | Except `_GLOBAL_OFFSET_TABLE_`, which older assemblers record and the linker defines |
| Non-empty allocated sections (name and flags) | Older assemblers emit empty `.data`/`.bss` |
| References (section, form, symbol) | `PC32`/`PLT32` are one form and the `GOTPCREL` variants another, as the linker treats them; a reference made directly in the original and through the GOT in the replay is accepted (the GOT form is the more general one), the reverse is not; counts (duplicated call sites) are ignored |

Results:
- **identical-bytes:** the allocated sections are byte-identical.
- **equivalent:** everything above matches, but the bytes differ.
- **accepted:** a difference reviewed and listed in
  [`qualification/accepted-differences.tsv`](../qualification/accepted-differences.tsv),
  matched by release, object, compiler family and exact difference.
- **differs** or **build-failed:** fails the qualification.

## Results (2026-10-11)

Local: WSL Ubuntu 24.04, GNU binutils 2.42.

| Compiler | Objects (28 releases) | identical-bytes | equivalent | accepted | differs | build-failed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| GCC 13.3 (Ubuntu) | 491 | 374 | 112 | 5 | 0 | 0 |
| Clang 18.1.3 (Ubuntu) | 507 rows | 136 | 104 | 0 | 49 | 218 |

**GCC** reproduces every release:
- 2.37–2.44 are byte-identical throughout;
- the equivalent objects differ only in instruction selection and relocation
  encodings;
- the five accepted rows are 2.21–2.25's `warning-nop.oS`. Their source declares a
  link-warning alias only `#if !__GNUC_PREREQ (5,0)`; the builder was GCC 4.8.
  Only code compiled by GCC older than 5 references it.

**Clang** cannot faithfully compile these files for 2.17–2.42.
- **The cause:**
  - glibc's internal headers give functions such as `__stat` an `extern inline` body.
  - The source then adds `attribute_hidden` or a weak hidden alias. GCC applies such
    late attributes; Clang ignores them ("attribute declaration must precede
    definition").
  - The result would be global, visible `stat`/`__stat` and similar instead of
    hidden, weak symbols, so programs and shared libraries would export them. The
    replay therefore makes this warning an error.
- **Other differences:** Clang writes glibc's `.gnu.warning` sections as allocated
  sections with a malformed name (glibc relies on a GCC-specific section-flag trick).
- **2.43–2.44:** all objects build; the only difference is `_IO_stdin_used` placed
  in `.rodata` instead of the mergeable `.rodata.cst4`.

glibc itself is only supported with GCC, so this result is expected.

**Decision (2026-10-11):** linux-abi-tools' `source` mode compiles with the host's
GCC. The full reasoning, including what goes wrong with Clang, is in
[PROVIDER-DELIVERY.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDER-DELIVERY.md#why-source-mode-compiles-with-gcc-not-clang).

## GCC versions (CI, 2026-10-12)

The [Replay workflow](../.github/workflows/replay.yml) rebuilds all 28 releases with
each compiler below on every push and weekly, and compares them with the recorded
interfaces ([run 38191162456](https://github.com/rustcommons/glibc-provider/actions/runs/38191162456)). Upstream GCC comes from the official `gcc`
images (no distribution defaults); distribution GCC comes from each distribution's
own packages, with its defaults.

| Compiler | binutils | identical-bytes | equivalent | accepted | differs | build-failed |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| GCC 11.5.0 (Rocky Linux 9) | 2.35.2 | 374 | 112 | 5 | 0 | 0 |
| GCC 12.5.0 | 2.40 | 374 | 112 | 5 | 0 | 0 |
| GCC 13.5.0 | 2.40 | 374 | 112 | 5 | 0 | 0 |
| GCC 13.3.0 (Ubuntu 24.04) | 2.42 | 374 | 112 | 5 | 0 | 0 |
| GCC 14.2.0 (Debian 13) | 2.44 | 366 | 120 | 5 | 0 | 0 |
| GCC 14.4.0 | 2.44 | 366 | 120 | 5 | 0 | 0 |
| GCC 15.3.0 | 2.44 | 366 | 120 | 5 | 0 | 0 |
| GCC 15.3.1 (Fedora 43) | 2.45.1 | 365 | 121 | 5 | 0 | 0 |
| GCC 16.2.0 | 2.44 | 366 | 120 | 5 | 0 | 0 |

Every version reproduces every release.
- From GCC 14, `crt1.o` of 2.37–2.44 is equivalent rather than byte-identical
  (instruction selection).
- Fedora 43 also gives an equivalent 2.44 `static-reloc.o`.
- Fedora's assembler adds an x86 "ISA used" `.note.gnu.property` to every object.
  The first run therefore failed every object, until the replay switched that
  default off like the other distribution defaults.

Build failures per release (Clang 18):

| Releases | Objects that fail to build |
| --- | --- |
| 2.17–2.20 | 11 |
| 2.21–2.31 | 12 |
| 2.32 | 14 |
| 2.33–2.34 | 2 |
| 2.35–2.42 | 3 |
| 2.43–2.44 | none |

## Not yet covered

- **Other GCC versions:** GCC older than 11 and versions not listed above are not
  qualified. Clang is not repeated: the attribute problem is a language difference,
  not a version difference.
- **Full suites:** the object comparison is necessary but not sufficient. The
  replayed provider must also pass linux-abi-tools' application, exhaustive-link and
  Cargo suites, as every release does in its provider workflow
  ([delivery modes per release](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDERS.md#delivery-modes-per-release)).
