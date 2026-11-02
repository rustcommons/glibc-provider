# Provider bundle unwind-impact release checklist

**Draft release gate; no new bundle or unwinder is qualified by this document.**
Use this source-repository checklist together with the canonical Cargo
[unwinder release checklist](https://github.com/rustcommons/cargo-libc/blob/feat/llvm-executable-unwinder/docs/UNWINDER-RELEASE-CHECKLIST.md)
(currently [draft PR #14](https://github.com/rustcommons/cargo-libc/pull/14), not main).
The version inspected on 2026-11-02 is
[`59006f5`](https://github.com/rustcommons/cargo-libc/blob/59006f559a2116c86b6b26f87a60187b4c9a270d/docs/UNWINDER-RELEASE-CHECKLIST.md).
Pin the actually reviewed canonical revision in each candidate record.

The maintainer's decision between an explicitly qualified LLVM mode and delaying
LLVM-default is **pending**. Native C cleanup/cancellation regression evidence is
a blocker to treating an LLVM default as safe. This draft neither selects a
runtime policy nor implements one. The canonical draft's baseline/diagnostic
commands are not the final changed-bundle qualification driver.

## Trigger and blocking rule

Require a recorded downstream unwind-impact assessment for every new or changed
glibc source/header/library/compile-command bundle, pack regeneration, or
replay/compiler/provenance update. Include changes to object interfaces, public
library definitions and builder defaults. There is **no data-only exemption**.
A runtime, toolchain, linker or provider-route change also invalidates evidence
whose recorded identities or assumptions no longer match.

Do not qualify or release an affected bundle until its exact candidate is
consumed by the tested producer/consumer, the applicable matrix below has fresh
results, and both owners review the disposition. Missing driver/API, unknown
symbol ownership, unexplained failures, or absent required runtime evidence
block the relevant qualification/release claim. A baseline-supported native C
cleanup or C-unwind regression must remain a failure, with raw evidence and an
explicit owner decision; Rust-only passes cannot waive it. Required skips need
an exact reason, bounded claim and recorded scope approval, not an implicit pass.

A documentation-only candidate may reuse bounded evidence when hashes/diffs
prove the affected package/runtime inputs byte-identical and the record explains
why the earlier execution applies. Prove source/data/pack, accepted differences,
producer/consumer/runtime configuration identity as relevant; a docs label alone
is insufficient. If a newer-than-host glibc cannot run, record inspection/link
results separately from the execution gap; do not claim runtime coverage.

For each new glibc release, review relevant **upstream runtime unwind changes**
against a pinned upstream revision/archive, including `misc/unwind-link.c`,
`nptl/unwind.c`, pthread_exit/cancellation/backtrace paths, handle-local dlsym and
SONAME selection, and target distribution configuration/patches. Record the exact
upstream ref, reviewed diff/paths, findings and reviewer, plus any uncovered
actual-target-runtime execution gap. This bundle contains minimal startup,
nonshared and header inputs, not full glibc: unchanged collected files or ABI
exports cannot establish unchanged runtime unwind behavior. Host-SDK tests are
not old-target execution. Unavailable Tier 3 execution remains explicitly deferred
and is not made a routine blocker; keep the actual evidence and claim bounded.

## Candidate evidence record

Save an immutable record per candidate, with raw logs/artifacts and these fields.
An unfilled field is pending, not inferred from source presence or old CI.

| Evidence | Required concrete fields |
| --- | --- |
| Source and bundle | Repository commit and clean/dirty state; glibc releases changed; pinned GNU archive digests and upstream signature evidence; source/header/table/pack hashes and byte diff from the released baseline. |
| Collection/provenance | Collector and pack-tool refs, qualified build-tree/container image digest, build logs, builder macros/defaults, kernel UAPI/header origins, compile commands and installed-target hashes. |
| References | interfaces.tsv/libraries.tsv digests, matching ABI-list ref/digest, accepted-difference file hash and exact reviewed reasons. No tolerance widening to hide unwind differences. |
| Producer and consumer | Exact linux-abi-tools/libc-abi-tools and cargo-libc source refs, versions, registry/path resolution and lock/metadata identities; prove the changed bundle actually feeds the tested provider. |
| Candidate injection | Reviewed disposable-copy input contract, expected first-party checkout SHAs, before/after manifests/locks/metadata and fixed unrelated registry checksums; no production manifest/lock override or invented checksum. |
| Runtime | Selected runtime and selection policy; LLVM revision/source/archive/member digests, patches/configuration/notices, architecture/PIC/EH/thread/visibility settings, or exact baseline runtime identity. Pending LLVM choice is not an approved default. |
| Toolchain and host | Rust/std/panic strategy, Clang/GCC, actual effective linker and version, build tools, host/target triple/features, native environment/container and loader/interpreter. A GNU label does not prove BFD if arguments select LLD. |
| Floor, kind and route | glibc floor and execution environment; output kind; source/container, explicit-provider or full-sysroot route; provider/cache/profile/content identity and cold/warm state. |
| ELF and ownership | Final ELF and link-map hashes; selected archive members; undefined/defined symbols, visibility, GNU symbol versions and resolved owners; ordered NEEDED, transitive closure and workload-observed lazy loads; EH/relocation/TEXTREL and search-path evidence. |
| LLVM libc impact | Actual LLVM-used libc imports attributed to members, required versions and owning DSOs; availability at 2.17, consumer 2.22 and every added release. Preserve non-unwind compiler helpers and investigate personality/context/resume ownership, including __gcc_personality_v0 and _Unwind_Resume when imported. |
| Execution | Exact driver/fixture revision, invocation and environment, timeout, command/stdout/stderr/exit or signal, binding/load traces and artifact hashes for each applicable case. Distinguish fresh candidate runs, baseline controls, reused evidence and source inspection. |
| Disposition | Per-case pass/fail/skip/pending, reason and scope; producer/consumer reviewer identities and dated approval; unresolved runtime-policy decision, driver/API gaps and explicit release-blocking failures. |

## Downstream verification matrix

Use the canonical acceptance IDs, not new local runtime contracts. Run relevant
cases with fresh candidate inputs and matched baseline controls. Cover native
x86_64 GNU BFD and LLD, Rust 1.90 plus the supported current toolchain, relevant
debug/optimized outputs, glibc 2.17 and consumer 2.22 controls, and each added
release/floor/claimed route. Record the actual environment and every coverage gap.

| Area | Required relevant checks and evidence |
| --- | --- |
| Panic, threads and backtrace (U01–05) | Return/catch, nested cleanup/resume and once-only destructors, panic/join/concurrent threads, first/repeated/threaded useful backtraces with transient loads, isolated uncaught/double-panic status. |
| Native C cleanup and C-unwind (U10–14) | C-only pthread_exit/deferred cancellation controls with both plain and -fexceptions objects; cleanup-bearing executable/DSO round trips returning to the originating catch; personality/context/resume/helper bindings. Preserve actual crashes and baseline successes. Do not send cancellation through Rust or unwind through ordinary C ABI. |
| Shared libraries and Python (U07–12) | Native host loads Rust cdylib, catches/translates errors and remains usable; two DSOs/load orders/visibility; supported Python/PyO3/maturin normal/error/panic conversion and reuse. Separate selected DSO artifacts from actual process symbol bindings. |
| Output kind and host isolation (U06/U19) | Executable/test/doctest/example/bench/install versus cdylib/dylib/rlib/staticlib, mixed bin+shared workspace, target C/CMake and host build/proc-macro; no executable-only policy contamination. |
| Linker and helpers (U14–16) | Member-attributed libc floor support, required non-unwind helpers and personality symbols; versioned refs; clock/state/order/archive/as-needed protections where affected. Twelve unwind imports alone are not complete native C coverage. |
| Routes, cache and negatives (U17–20) | Source/container, explicit provider and full sysroot; contained/read-only inputs; missing/corrupt/wrong-architecture/configuration/symlink/thin-member/cache refusal; concurrent generation; changed-bundle relink then no-op and stale-config refusal. |
| Consumer/package boundary (U21–22) | Fresh consumer comparison when affected inputs ship, preserving the canonical strict comparator and explaining genuine runtime import changes; exact package/registry resolution and final source identity. Do not claim Zig equivalence from a passing subset. |

Source collection, hash/pack checks, object replay and symbol inspection retain
their existing [maintenance](MAINTAINING.md) and [replay](REPLAY.md) roles. They do
not substitute for panic/native C/shared-library/Python execution. A lack of
direct libgcc_s NEEDED is not proof that libc, a DSO or a lazy load cannot bring
it in; do not claim universal no-libgcc behavior or hide fallback dependencies.

## Driver and decision handoff — pending

| Dependency | Current disposition / link needed before finalization |
| --- | --- |
| Runtime direction | Pending maintainer choice: explicit qualified LLVM mode versus delaying LLVM-default. Record the actual decision and supported scope; no safe-default assumption. |
| Producer API/driver | Pending exact reviewed candidate API and runnable linux-abi-tools driver link for archive/provenance, disposable bundle inputs, helper inspection, routes and cache identity. Do not invent invocation flags. |
| Consumer qualification driver | Pending exact reviewed cargo-libc full candidate/data-override driver link and commands implementing applicable U01–U22 with this producer API. PR #14 currently documents gaps; baseline/preflight is insufficient. |
| Native C/DSO/PyO3 commands | Pending final candidate commands/fixtures and runtime resolution model. The [draft diagnostic fixtures](https://github.com/rustcommons/cargo-libc/tree/59006f559a2116c86b6b26f87a60187b4c9a270d/tests/fixtures/unwind-coexistence) preserve failures; they are not release qualification. |

No consumer/producer command is prescribed here until those real interfaces and
candidate identities are delivered. Keep this checklist/GP-REQ-018 draft and the
affected qualification blocked while these handoffs or required evidence are
missing. Parent finalization must link actual drivers and the runtime decision.

## Existing boundaries

Never hand-edit `data/`, alter raw source/header bytes, or weaken provenance and
accepted-difference rules to work around an unwind failure. Preserve per-file
origins/licenses and pack integrity. Runtime redistribution review does not grant
prebuilt-provider clearance: compiled provider objects remain gated on legal
review. Tier 3 actual old-system qualification remains deferred, not a routine
maintenance blocker or an implied pass. Neither this checklist nor a docs-only
update changes those boundaries or authorizes publication; follow the
[release policy](RELEASE-PLAN.md#release-documentation-policy).
