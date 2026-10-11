# Rebuilding the objects locally

`glibc-provider-replay` rebuilds a release's startup objects and nonshared
archives from the recorded inputs with a local compiler, and compares every
object with the container build. This is the qualification for linux-abi-tools'
`source` delivery mode ([PROVIDER-DELIVERY.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDER-DELIVERY.md)).

```sh
glibc-provider-replay --release 2.28 --compiler gcc --out /tmp/replay-2.28 \
  --compare /tmp/glibc-2.28-work
```

## How a command is replayed

1. **Recreate the inputs.** The recorded archive files, generated headers and
   kernel UAPI headers are recreated from `data/blobs/`, each checked against its
   SHA-256.
2. **Run each recorded command:**
   - `gcc` is replaced by the chosen compiler, run from the recorded source
     subdirectory, with dependency-file options dropped.
   - `-nostdinc` is added, with only the compiler's own built-in headers and the
     recorded UAPI headers on the system include path. No host header is visible.
   - The builder compiler's implicit defaults, recorded from its predefined macros
     (`builder.*` in `release.tsv`), are made explicit wherever the command does not
     set them: `-fPIE` or `-fno-pie`, and no stack protector, CET or fortification.
     The replaying compiler's own distribution defaults are switched off in the same
     way (`-fcf-protection=none`, `-fno-stack-clash-protection`, `-U_FORTIFY_SOURCE`).
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

- **Other compiler versions:** these results are for one GCC and one Clang. Other
  GCC versions (12, 14, 15) and Clang versions need the same run; CI will run them
  for every release.
- **Full suites:** the object comparison is necessary but not sufficient. The
  replayed provider must also pass linux-abi-tools' application, exhaustive-link and
  Cargo suites (roadmap step "Qualify every release in every mode").
