# Requirements and documentation map

## Scope and sources

This index records confirmed contracts for `glibc-provider` and its published
`glibc-provider-sources` library, not new features or universal compatibility.
Sources of authority are [AGENTS.md](../AGENTS.md), the [manifest](../Cargo.toml),
the existing guides and dated decisions linked below. Implementation and tests
supply bounded evidence; their presence alone is not proof of a successful run.

Audit date: 2026-11-01. Source/CLI/workflow inspection used baseline
[`7f3a5d6`](https://github.com/rustcommons/glibc-provider/tree/7f3a5d6115f04ce80bfef2115fc42b9230202a0d).
GitHub reported successful [CI run 41238038842](https://github.com/rustcommons/glibc-provider/actions/runs/41238038842)
and [Replay run 41238038853](https://github.com/rustcommons/glibc-provider/actions/runs/41238038853)
for that exact revision (2026-10-31). References to those runs below are historical
baseline evidence, not new executions for this documentation change.

## Requirements

| ID | Requirement and source | Implementation / documentation | Verification evidence | Status and gaps |
| --- | --- | --- | --- | --- |
| GP-REQ-001 | Library and collector use std-only Rust, Rust 1.90 minimum; [AGENTS](../AGENTS.md#scope-and-implementation), [manifest](../Cargo.toml). | Empty external dependency sections in both manifests; library owns decoder and SHA-256; [README](../README.md#use-the-library). | Manifest and source inspection 2026-11-01; baseline CI `msrv` runs `cargo test --workspace --locked --offline` with 1.90.0. | Verified (bounded to manifests and baseline MSRV CI); no claim for every platform. |
| GP-REQ-002 | Recorded scope is x86_64 Linux, glibc 2.17–2.44, startup objects and nonshared archives including pthread before 2.28; [AGENTS](../AGENTS.md#project-overview). | [RELEASES, TABLES and object API](../src/lib.rs); [targets](DATA.md#targets). | Baseline CI: `tests/bundled.rs::bundled_pack_holds_every_release_with_its_inputs` checks release list, targets and pthread metadata consistency; inspection of collector target selection. | Verified (recorded dataset and named checks); other architectures/releases are outside this contract. |
| GP-REQ-003 | Expose exact recorded source/header inputs, commands, builder defaults and interface/library metadata; [DATA](DATA.md). | `Sources`, `Release`, `SourceFile`, `Header`, `Object`, seven tables; [API reference](https://docs.rs/glibc-provider-sources/0.1.0/glibc_provider_sources/). | Baseline bundled test checks interface digest counts, eight SONAMEs, headers and defaults; source inspection of `Release` table readers. | Implemented—partial verification; structural checks do not independently recollect every command or prove all metadata semantically correct. |
| GP-REQ-004 | Decode and hash bundled inputs without compiler, external decompressor or container; [README](../README.md#use-the-library). | `Sources::bundled/from_gzip/parse`, `src/inflate.rs`, `src/sha256.rs`. | Baseline CI library tests including `rejects_corrupt_packs`, `stores_later_versions_as_deltas`; 2026-11-01 inspection of decoder calls and hash/size checks. | Verified (named parser tests and implementation inspection); not a general security audit. |
| GP-REQ-005 | Extraction requires new destination directories with existing parents, preserves class layout and can leave partial output on failure; [DATA](DATA.md#using-the-library). | `Release::write/write_sources/write_headers`; compiler headers omitted; installed headers go directly under destination. | Baseline `reads_and_writes_a_release` rejects repeated header destination; `writes_a_release_tree_with_the_recorded_contents` compares extracted source bytes for 2.17; inspection of `fs::create_dir` and writes. | Verified (named cases and inspected failure semantics); no transactional-write guarantee. |
| GP-REQ-006 | Use qualified provider work trees and pinned signature-checked GNU archives; archive bytes must match the trusted pin and pristine extraction; never hand-edit data; [AGENTS](../AGENTS.md#scope-and-implementation). | Collector `collect`, `Collector::object/target`; [maintenance prerequisites](MAINTAINING.md#prerequisites-and-working-directories). | 2026-11-01 inspection of archive SHA-256, fresh tar comparison, installed-target comparison and archive-member byte matching; signature verification belongs to upstream pipeline. | Implemented—partial verification; no fresh collector run or independent upstream signature verification in this audit. |
| GP-REQ-007 | Preserve archive/generated/UAPI/compiler/builder classes, origins and licenses; compiler headers are path-only; [AGENTS](../AGENTS.md#scope-and-implementation), [NOTICE](../NOTICE.md). | [Class table](DATA.md#input-classes-inputstsv-filestsv), collector `classify/store/headers`, manifest license expression. | Baseline CI collector `classifies_inputs` and bundled checks; 2026-11-01 inspection of class handling, notice and package include list. | Implemented—partial verification; no new legal review or per-file license recertification. |
| GP-REQ-008 | Pack must represent all tables and stored files exactly; regenerate only for intended data changes, rebuild consumers to embed it; [AGENTS](../AGENTS.md#validation). | [Pack format](DATA.md#the-crates-pack), [sequencing](MAINTAINING.md#pack-rebuild-and-replay), collector pack `--check`. | Baseline CI `pack` job runs `glibc-provider-pack --check`; `data` job checks blob hashes/references. Source inspection confirms decoded-content equality, not compressed-byte equality. | Verified (baseline pack/data jobs); no data regeneration or new pack check run in this audit. |
| GP-REQ-009 | Record interfaces and public library definitions from qualified builds, without copying binary code/data; [DATA](DATA.md#interfacestsv), [library table](DATA.md#librariestsv). | Replay `--record`; downstream `libc-abi glibc libraries`; [recording guide](MAINTAINING.md#record-reference-interfaces-and-libraries). | Baseline `recorded_interfaces_read_back_unchanged` test; 2026-11-01 inspection of record-mode CLI, table schemas and documented stdout handling. | Implemented—partial verification; no new reference recording or downstream library extraction. |
| GP-REQ-010 | Replay recorded commands with explicit defaults, isolated headers and defined interface comparison; accept only exact reviewed differences with reasons; [AGENTS](../AGENTS.md#scope-and-implementation), [REPLAY](REPLAY.md). | `collector/src/replay.rs`; [accepted differences](../qualification/accepted-differences.tsv); report status vocabulary. | Baseline Replay run cited above; CI tests `compares_references_not_encodings`, `makes_builder_defaults_explicit_only_where_unset`, `translates_commands_for_another_compiler`, `accepted_differences_name_recorded_objects`. | Verified (baseline matrix and named checks); not evidence for all GCC versions or Clang. |
| GP-REQ-011 | Source delivery uses GCC; compiler qualification and downstream application/link/Cargo suites have distinct scopes; [dated decision](REPLAY.md#results-2026-10-11). | [Replay limitations](REPLAY.md#not-yet-covered), [Replay CI](../.github/workflows/replay.yml), downstream provider workflow. | Historical 2026-10-11/12 reports in REPLAY plus baseline Replay CI; 2026-11-01 inspection of floating image matrix. | Implemented—partial verification; this audit did not rerun replay or downstream suites and does not broaden qualification. |
| GP-REQ-012 | Package sources and metadata, with complete license inventory; collector stays unpublished and no glibc binary objects ship; [NOTICE](../NOTICE.md), [manifest](../Cargo.toml). | Include list; collector `publish = false`; [release gates](RELEASE-PLAN.md#gates-for-an-authorized-release). | Baseline CI packages and checks 9,000,000-byte ceiling; 2026-11-01 inspection of manifest, NOTICE and registry 0.1.0 README/API source. | Verified (bounded package checks); future package candidates need their own validation. |
| GP-REQ-013 | Prebuilt distribution requires legal review before any binary data enters the public repo; [ROADMAP](ROADMAP.md#data-crates). | Sources-only distribution; proposed `glibc-provider-binaries` has no approved implementation. | 2026-11-01 inspection of roadmap legal gate and package contents policy. | Planned; legal gate unchanged, no binary distribution authorized. |
| GP-REQ-014 | Tier 3 old-system qualification remains deferred; [roadmap](ROADMAP.md#deferred-qualification) and downstream validation record. | Downstream qualification work, separate from object replay. | Coordinator scope confirmed 2026-11-01; no new old-system runs in this audit. | Deferred; not a routine maintenance blocker and not an implied compatibility claim. |
| GP-REQ-015 | README provides purpose, prerequisites, setup and guide links; immutable package docs require separate approved release to change; [release plan](RELEASE-PLAN.md#what-reaches-consumers). | [README](../README.md), maintained guides, versioned API docs. | Previous maintenance record supplied for baseline `7f3a5d6`: README example compiled with Rust 1.90; not repeated here. Baseline CI tests API doctest. Registry archive inspected 2026-11-01; API source matches checkout. | Implemented—partial verification; published 0.1.0 README has real stale status/commands/links recorded in the release plan. Git fixes do not replace it. |
| GP-REQ-016 | Validate affected docs and preserve release/provenance boundaries; publication needs explicit approval; [AGENTS](../AGENTS.md#validation), [release plan](RELEASE-PLAN.md#gates-for-an-authorized-release). | [CI](../.github/workflows/ci.yml), maintenance/release checklists and this stable trace index. | 2026-11-01 source/CLI-option inspection; PR records link, Markdown rendering and diff checks. No Rust examples/source changed; unrelated Rust tests/data jobs not run locally. | Implemented—partial verification; shared conventions link awaits central merge and final cross-repo link review. |

## Documentation map

| Document / surface | Class | Authority and maintenance boundary |
| --- | --- | --- |
| [README](../README.md) | Maintained guide | Setup-first entry point; absolute GitHub links reach excluded guides from package rendering. |
| [DATA](DATA.md) | Maintained reference + historical evidence | API use, extraction and formats; explicitly dated totals are measurements, not fresh verification. |
| [MAINTAINING](MAINTAINING.md) | Maintained guide | Authoritative collection/record/pack/replay sequence and prerequisites. |
| [REPLAY](REPLAY.md) | Maintained reference + historical evidence | Commands and comparison contract; dated local/CI results retain their original scope. |
| [RELEASE-PLAN](RELEASE-PLAN.md) | Maintained release guide + planned + historical evidence | Current gates and package defects; older version/dependency snapshots remain labeled. No release authority granted by a plan. |
| [ROADMAP](ROADMAP.md) | Planned + deferred | Prebuilt legal gate and deferred Tier 3; no implementation/qualification promises. |
| [NOTICE](../NOTICE.md), [LICENSES](../LICENSES), [REUSE](../REUSE.toml) | Maintained reference | Provenance/license inventory; not permission to bypass legal review. |
| [Rust API source](../src/lib.rs) / [0.1.0 API](https://docs.rs/glibc-provider-sources/0.1.0/glibc_provider_sources/) | Maintained reference / immutable snapshot | Public items and doctest; published version stays fixed. No source changes in this audit. |
| [CI](../.github/workflows/ci.yml), [Replay workflow](../.github/workflows/replay.yml), [qualification script](../qualification/replay-all.sh) | Maintained executable reference | Workflow definitions are not run outcomes; exact runs cited above supply historical evidence. |
| [AGENTS](../AGENTS.md), [CLAUDE](../CLAUDE.md) | Maintained maintainer reference / shim | AGENTS is primary; shim points to it, with no reverse authority delegation. |
| This index | Maintained reference | Confirmed contracts, stable GP-REQ IDs, bounded evidence and explicit gaps. |

## Maintenance

Follow the [shared documentation conventions](https://github.com/rustcommons/.github/blob/main/docs/DOCUMENTATION.md)
and the primary AGENTS rules. The shared URL is pending central merge as of this
audit; verify it and cross-repo links before merge. Keep existing guide paths.

Update affected rows and guides together. Preserve GP-REQ IDs, add without
renumbering/reuse, and retain dated evidence instead of silently replacing old
outcomes. New evidence must identify the exact revision, command/test/CI run and
scope. Source inspection, test presence, object replay and downstream system
qualification are different evidence. Recheck live registry state for an actual
release; this audit does not authorize publication or treat trace links as a
reason to publish. No existing feature IDs were replaced by this index.
