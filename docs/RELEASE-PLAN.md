# Release preparation and historical plan

Updated: 2026-11-02. `glibc-provider-sources` **0.1.1 is published**, non-yanked
at inspection, and delivers the README corrections. No API, behavior, input,
compiler-qualification or license scope changed. External consumers remain unknown.

## Release documentation policy

For `glibc-provider-sources`, maintain the root [changelog](../CHANGELOG.md) and
[versioned user-facing notes](releases/) alongside this qualification procedure.
Record completed user-visible changes under Unreleased, then give each shipped
version its original date, changes, upgrade impact, validation/limits and links.
Future proposals stay out of shipped entries. Before publication, commit
`docs/releases/vX.Y.Z.md` and run the release-documentation CI check in addition
to the candidate-specific package/provenance checks below.

The stdlib-only checker requires Python 3.11 or newer:

```sh
python3 scripts/check-release-docs.py --repo rustcommons/glibc-provider
```

The local check needs no network. The [release-documentation workflow](../.github/workflows/release-docs.yml)
runs it and supports manual dispatch on a candidate branch once registered on the
default branch. On first installation, independently review the local checker
and syntax evidence, then use an approved normal merge to register the workflow.
A green default-branch documentation run is mandatory before bootstrap completion
or any GitHub Release note publication; future routine merges require the
registered CI check. The initial candidate dispatch on 2026-11-02 returned 404
because the workflow was absent on the default branch; no candidate CI pass is
claimed. After GitHub Release
backfill, run the checker with `--published` (network required) to compare all
actual release bodies with committed notes and verify the latest stable version;
release events verify their tag against current default-branch documentation.
That distinction lets old source tags retain their original contents.

A release is complete only when the immutable source tag, registry archive,
GitHub Release notes, changelog, API/user docs and required verification agree.
GitHub Releases document source and registry availability; distribution artifacts
remain on crates.io. No standalone binaries or other new attachments are
authorized by this policy. The prebuilt compiled-provider legal gate is unchanged.
Publish reviewed notes from their committed file against the existing tag with
`gh release create --verify-tag --notes-file`; use absolute links, and verify the
live release text, links, exact tag and latest status afterward. Do not move tags
or falsify GitHub publication dates. Historical versions must not replace the
actual newest stable release as latest.

At the initial 2026-11-02 inspection, v0.1.0 and v0.1.1 had source tags and
published crates but no GitHub Release entries. The notes and changelog were
backfilled from source/registry records. These files were absent at the historical
tags: link to current main or the backfill documentation commit, never to a
nonexistent changelog at an old tag. Original crate dates remain distinct from
the documentation backfill and GitHub Release publication dates. The initial
registration/404 observations above are historical; completion is recorded below.
No new package version was created and immutable READMEs were not replaced.

## GitHub Release backfill completed (2026-11-02)

- Reviewed bootstrap [PR #4](https://github.com/rustcommons/glibc-provider/pull/4)
  merged normally at
  [`a30d40e`](https://github.com/rustcommons/glibc-provider/commit/a30d40ee166d9ce08756578560cef802907c058c),
  preserving the reviewed commits and tree. The registered workflow's
  [bootstrap run 41387835086](https://github.com/rustcommons/glibc-provider/actions/runs/41387835086)
  succeeded on that exact merge before either Release entry was published.
- [v0.1.0](https://github.com/rustcommons/glibc-provider/releases/tag/v0.1.0)
  was documented at `2026-11-02T04:20:30Z`, followed by
  [v0.1.1](https://github.com/rustcommons/glibc-provider/releases/tag/v0.1.1)
  at `2026-11-02T04:20:31Z`, from their committed versioned notes using existing
  verified tags. These are GitHub documentation dates, not the original registry
  publication dates.
- Full local `--published` verification passed, and the workflow's
  [full verification run 41387968372](https://github.com/rustcommons/glibc-provider/actions/runs/41387968372)
  succeeded with `verify_published=true` on `a30d40e`. Both published bodies match
  their committed note files exactly (after the checker's newline normalization).
  v0.1.1 is latest; both entries are non-draft, non-prerelease and have **zero
  uploaded assets**.
- Original source tags are unchanged: v0.1.0 identifies
  `1ae578cf7123399b0c710f44defab93cc2021001`; v0.1.1 identifies
  `2a7a0fafce50b8516208a89c98f7b5023b77ecbf`. No crate publication, retagging,
  binary distribution or new Rust/replay qualification occurred in this backfill.

The bootstrap and live-note verification gap is closed for these two versions.
Future routine merges and release completion still require the registered
release-documentation checks. All earlier dated package/validation records retain
their original bounds.

## 0.1.1 published release (2026-11-01)

- Released commit: [`2a7a0fa`](https://github.com/rustcommons/glibc-provider/commit/2a7a0fafce50b8516208a89c98f7b5023b77ecbf),
  the normal merge of [PR #1](https://github.com/rustcommons/glibc-provider/pull/1).
  The approved reviewed commits are preserved. Annotated
  [`v0.1.1`](https://github.com/rustcommons/glibc-provider/tree/v0.1.1) peels exactly
  to that commit (tag object `d6d2c85d98b4311683ec6b5bebb1f2c347bfaef7`).
- The coordinator published; the [registry version API](https://crates.io/api/v1/crates/glibc-provider-sources/0.1.1)
  records publication at `2026-11-01T04:38:44.852582Z`, non-yanked when checked.
  Official archive SHA-256:
  `da10605d1cab20eb0ab8cf7ebe19b619737618578dc8c74e6c62a57e1a7ec440`;
  size **4,770,985 bytes**, below the 9,000,000-byte gate, with **18 files**.
  Its checksum matches the registry. The coordinator independently audited the
  official archive at `2026-11-01T04:39:30.8213138Z` with the same checksum,
  released commit, non-yanked status and file count.
- Official `.cargo_vcs_info.json` identifies the exact release commit, root
  repository path and no dirty marker. All 18 extracted file contents match the
  clean merged package verified with Rust 1.90.0. The local archive checksum
  `11c9bc523ee7a0d0066ba13d836552d82880227d475c424260c675afb2267e4b`
  and size 4,770,999 bytes differ from the published archive's container bytes;
  extracted file equality, rather than archive-byte equality, was verified.
- All **13 protected payload files** match official 0.1.0 byte for byte: Rust
  modules/decoder fixture, source pack, accepted differences, NOTICE and five
  license texts. Both manifest semantic comparisons differ only in library
  version; the lock change is also only the library version. No API/source,
  source blob/table, collector, replay tolerance or license change was made.
  The same 18-file include inventory excludes raw data, collector, tests and guides.
- [docs.rs build #5104129](https://docs.rs/crate/glibc-provider-sources/0.1.1/builds/5104129)
  completed at `2026-11-01 04:39:14` using rustc
  `1.101.0-nightly (f9ceb4a4d 2026-10-31)`. The log records generated
  x86_64-unknown-linux-gnu docs, and the [versioned API](https://docs.rs/glibc-provider-sources/0.1.1/glibc_provider_sources/)
  returns 200 with the public items. This is documentation-build evidence, not
  new runtime or replay qualification.
- The actual [crates.io 0.1.1 README](https://crates.io/crates/glibc-provider-sources/0.1.1)
  and [docs.rs package README](https://docs.rs/crate/glibc-provider-sources/0.1.1)
  were inspected in a browser. The library now says “This repository publishes”,
  with Rust 1.90 prerequisites and the complete example. “Not yet published”, the
  obsolete collector command and invalid hash placeholder are absent. All guide
  links rendered as absolute GitHub URLs; DATA, REPLAY, MAINTAINING, REQUIREMENTS,
  RELEASE-PLAN, ROADMAP and NOTICE links returned 200, including DATA's extraction
  anchor. The official archive README equals the corrected released README.
- The earlier direct-HTTP crates.io 404 observation is historical: a direct fetch
  still returned 404 during this check, but the real browser successfully loaded
  the versioned registry page and rendered the corrected README. It is not a
  missing-publication or current reader-rendering gap.

Current registry inspection and coordinator evidence also confirm
`glibc-abilists` 0.1.2, the five linux-abi-tools packages 0.3.3 and cargo-libc 0.2.2.
No downstream lock refresh or additional release is needed for this documentation
patch. Published 0.1.0 remains an immutable historical snapshot with its original
README defects; 0.1.1 delivers the corrections now. Prebuilt compiled provider
objects remain Deferred pending legal review; Tier 3 remains deferred and is not
a routine blocker. The exact Rust/MSRV/Clippy and baseline replay evidence below
retain their original bounds; no fresh local replay qualification is claimed.

## 0.1.1 documentation release candidate

Historical preparation record, completed by the published release above.

PR [#1](https://github.com/rustcommons/glibc-provider/pull/1), branch
`docs/requirements-audit`, also carries the documentation audit and GP-REQ index.
The release changes the published library version and its workspace lock entry
from 0.1.0 to 0.1.1. The unpublished collector stays 0.1.0; its unversioned local
path dependency needs no minimum change. No dependencies, API, behavior, source
blobs, tables, pack, accepted differences or licenses change. The pre-existing
manifest comment referring to AGENTS instead of the shim is a documentation-only
change since official 0.1.0, not a package policy change.

The packaged README now identifies a published library, provides Rust 1.90
prerequisites and a complete non-writing example, and uses absolute GitHub guide
links because `docs/` is excluded from the crate. Collector regeneration commands
and the real pinned-hash prerequisite live in the corrected maintenance guide,
not as invalid README shell commands. This repairs actual 0.1.0 package defects;
trace navigation alone is not the reason for release.

The exported API and embedded inputs must match the official registry 0.1.0
archive byte for byte. Existing downstream 0.1.0 locks need no update for this
documentation-only release. Do not trigger linux-abi-tools or cargo-libc releases
for guide links. No collector regeneration or broad replay matrix is needed for
unchanged inputs; the prebuilt legal gate and deferred Tier 3 remain unchanged.

### Candidate validation (2026-11-01)

The code/package-input revision checked locally is
[`8fda7f4`](https://github.com/rustcommons/glibc-provider/commit/8fda7f4edbb5c97fb267f82d2d8dbd71d39d221d).
Subsequent evidence edits affect only excluded `docs/` records. Environment:
aarch64 Linux, at most two Cargo build jobs; this is not x86_64 object replay.

- Rust 1.90.0 (`1159e78c4`): `cargo +1.90.0 fmt --all -- --check` and
  `cargo +1.90.0 test --workspace --locked --offline` passed: 28 unit/integration
  tests and one API doctest. These test scopes do not certify all host systems.
- Clippy 1.90.0 with `--workspace --all-targets --locked --offline -- -D warnings`
  failed on the existing `nonminimal_bool` expression at `collector/src/main.rs:133`.
  That source is identical to official 0.1.0 and was not changed or suppressed.
  The required Clippy gate passed with CI's pinned Rust 1.99.0 (`b940084d7`):
  `cargo +1.99.0 clippy --workspace --all-targets --locked --offline -- -D warnings`.
  MSRV compilation/tests and CI-version lint policy are distinct checks.
- `RUSTDOCFLAGS='-D warnings' cargo +1.90.0 doc --no-deps --workspace --locked --offline`
  passed. API source and README Rust code blocks are unchanged from the audited
  baseline; candidate artifact/example verification is recorded with the package
  audit in [PR #1](https://github.com/rustcommons/glibc-provider/pull/1).
- `cargo +1.90.0 run --release --locked --offline -p glibc-provider-collector
  --bin glibc-provider-pack -- --check` passed: 9,830 entries, 4,747,631 compressed
  bytes, decoded contents equal `data/`. No pack regeneration or collector/replay
  qualification was run locally.
- The official registry 0.1.0 archive SHA-256 matches its API checksum:
  `62eb50dd168d92ce36af896177c886c7889b1f62f5f09d07d64bedff74dac04e`.
  All 13 protected package files (excluding README, Cargo and VCS metadata) match
  this checkout byte for byte. Git diff against `v0.1.0` is empty for `src/`,
  `data/`, `pack/`, `qualification/`, `collector/`, `LICENSES/`, `NOTICE.md`,
  `REUSE.toml` and `tests/`. No API/source-comment, input, tolerance or license
  change is hidden by the documentation release.
- GitHub Markdown rendering, local link/anchor and schema checks passed. The
  shared standard now returns HTTP 200 and Contents API confirms blob
  `1b659ff536d43bcae5b09626069e89488058ba6f` at central commit
  `c8a0b9c01aae5657dd0ef649bd37b18d81919ce5`; the initial pending-link observation
  is resolved. The canonical crates.io page still returns 404 although registry,
  archive and docs.rs work; that discrepancy remains explicit below.

After the final evidence commit, the package audit in PR #1 records
`cargo +1.90.0 package -p glibc-provider-sources --locked --offline`, its verification
build, the exact artifact checksum/size (ceiling 9,000,000 bytes), included file
and license inventory, VCS identity, payload comparison and compilation/execution
of the packaged README example on Rust 1.90.0. It identifies the exact candidate
commit; branch package VCS metadata must not be mistaken for the later merge commit.

At this preparation checkpoint, independent reviewer GO and parent normal merge
approval were still required, followed by coordinator publication and verification.
Those steps are completed in the published release record above; this historical
checkpoint alone was not publication evidence.

## Historical decision and scope (2026-10-31)

Repository documentation cleanup does not by itself justify a release. Keep
`glibc-provider-sources` at 0.1.0 until there is an approved packaged change.
If the next change is only packaged README/API documentation or a compatible fix,
recommend **0.1.1**, conditional on review and the gates below. Consider **0.2.0**
for incompatible API, pack-format, target-support, or observable data-contract
changes; do not classify a data regeneration as a patch merely because signatures
are unchanged. Review additions and changed provider output explicitly.

The documentation scope includes agent-instruction consolidation, this plan,
README library setup, and extraction/replay/maintenance guides. No data,
pack, API, compiler qualification, dependency, or license changes are proposed.
The unpublished MIT collector remains a workspace tool, not a second public
release. `glibc-provider-binaries` remains blocked on the legal review described
in [ROADMAP.md](ROADMAP.md); do not add its binary data ahead of that review.

## Pre-publication documentation inspection (2026-11-01)

The [requirements index](REQUIREMENTS.md) is the current map of contracts,
documentation and bounded evidence. Before the documentation releases, registry
APIs inspected on 2026-11-01 reported
`glibc-provider-sources` 0.1.0, `glibc-abilists` 0.1.1, all five linux-abi-tools
packages 0.3.3 and cargo-libc 0.2.2. The completed downstream releases are recorded
at [linux-abi-tools v0.3.3](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.3)
and [cargo-libc v0.2.2](https://github.com/rustcommons/cargo-libc/tree/v0.2.2).
Do not repeat them. See the downstream [release procedure](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/RELEASING.md)
for future authorized work. The older version and dependency observations below
remain historical evidence, not current rollout instructions.

Inspection of the registry's immutable 0.1.0 archive on 2026-11-01 found actual
packaged README defects: it says the crate is not published, links relatively to
excluded `docs/` files, and shows collector commands without `--bin` and with an
unusable shell hash placeholder. The current Git [README](../README.md) and
[maintenance guide](MAINTAINING.md) correct those instructions. Packaged
`src/lib.rs` is byte-identical to the audited baseline; no API misinformation
was identified in this inspection. Correcting the distributed README requires
an approved new package version; adding repository trace links alone does not
justify one. The initial audit proposed no version; the subsequent maintainer
instruction authorized the 0.1.1 preparation above for these real package defects.

## Historical baseline and evidence (2026-10-31)

Queried GitHub manifests, locks, tags, workflows, and crates.io on 2026-10-31.
The registry versions below are non-yanked; all older versions returned by these
crate APIs were also non-yanked at inspection time. This is a dated observation,
not a permanent guarantee or proof of current CI success. The registry APIs
confirmed publication, but the human-facing crates.io pages for these two glibc
crates returned HTTP 404 during link checks. Recheck registry-page rendering
before release; do not infer absence of the published versions from that response.

| Package | Latest published | Evidence |
| --- | --- | --- |
| glibc-provider-sources | 0.1.0 | [registry API](https://crates.io/api/v1/crates/glibc-provider-sources), [version](https://crates.io/api/v1/crates/glibc-provider-sources/0.1.0) |
| glibc-abilists | 0.1.1 | [registry API](https://crates.io/api/v1/crates/glibc-abilists), [version](https://crates.io/api/v1/crates/glibc-abilists/0.1.1) |
| libc-abi-tools | 0.3.1 | [registry API](https://crates.io/api/v1/crates/libc-abi-tools) |
| kernel-abi-tools | 0.3.1 | [registry API](https://crates.io/api/v1/crates/kernel-abi-tools) |
| linux-abi-scan | 0.3.1 | [registry API](https://crates.io/api/v1/crates/linux-abi-scan) |
| linux-abi-tools | 0.3.1 | [registry API](https://crates.io/api/v1/crates/linux-abi-tools) |
| linux-targets | 0.3.1 | [registry API](https://crates.io/api/v1/crates/linux-targets) |
| cargo-libc | 0.2.0 | [registry API](https://crates.io/api/v1/crates/cargo-libc) |

Local manifests and lock agree on sources/collector 0.1.0, Rust 2021, MSRV 1.90,
and no external Rust dependencies. The only provider tag is
[`v0.1.0`](https://github.com/rustcommons/glibc-provider/tree/v0.1.0), commit
`1ae578cf7123399b0c710f44defab93cc2021001`. The inspected provider main was
`dff7aae4f1b7e2c3f4151d88349e5812aa03092b`; changes since that tag were documentation.
Other latest tags verified were
[glibc-abilists v0.1.1](https://github.com/rustcommons/glibc-abilists/tree/v0.1.1),
[linux-abi-tools v0.3.1](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.1),
and [cargo-libc v0.2.0](https://github.com/rustcommons/cargo-libc/tree/v0.2.0).

This repository has [CI](../.github/workflows/ci.yml) and
[Replay](../.github/workflows/replay.yml), but no automated publishing workflow or
standalone release procedure. [AGENTS.md](../AGENTS.md) supplies its validation
and explicit publication-approval requirements. The downstream workspace has an
existing [release procedure](https://github.com/rustcommons/linux-abi-tools/blob/6c67fa61481a516edd020291085457a1b864be4a/docs/RELEASING.md).

## What reaches consumers

| Change | Delivery needed |
| --- | --- |
| GitHub description/topics, agent instructions, `docs/` guides or this plan | GitHub update only; these files are not in the crate include list. No downstream bump or rebuild. |
| Packaged README, NOTICE, license texts, manifest metadata | A new crate version is needed to change the distributed package/version's registry content. GitHub changes alone do not replace an existing package. |
| Rust API documentation in `src/` | Publish a new version for consumers and its docs.rs API pages. Editing GitHub guides does not update existing versioned API docs. |
| Rust behavior, `pack/sources.gz`, included accepted differences | Publish a reviewed version; downstream users must resolve it and rebuild to obtain the new embedded content. |
| Raw `data/` alone | Not shipped directly. Regenerate and verify the pack before any consumer delivery; never ship a stale pack. |

The [manifest](../Cargo.toml) includes source, pack, accepted differences, README,
NOTICE and license files, but excludes `docs/`, agent instructions, and collector.
Check README links as rendered on GitHub and crates.io; excluded guides must
remain reachable through repository URLs. Updating an already linked GitHub
guide can help readers immediately without republishing the crate.

## Historical first-party dependency and artifact map (2026-10-31)

Snapshot sources: linux-abi-tools
[manifest](https://github.com/rustcommons/linux-abi-tools/blob/6c67fa61481a516edd020291085457a1b864be4a/Cargo.toml)
and [lock](https://github.com/rustcommons/linux-abi-tools/blob/6c67fa61481a516edd020291085457a1b864be4a/Cargo.lock);
cargo-libc [manifest](https://github.com/rustcommons/cargo-libc/blob/ed7033ddd49f189d8b964cac510908ec56feffd2/Cargo.toml)
and [lock](https://github.com/rustcommons/cargo-libc/blob/ed7033ddd49f189d8b964cac510908ec56feffd2/Cargo.lock).
These snapshots do not include other agents' uncommitted work.

- `libc-abi-tools` requires `glibc-provider-sources = "0.1.0"` and
  `glibc-abilists = "0.1.0"`, both optional dependencies enabled by default through
  `provider-sources` and `bundled-glibc-data` respectively. Its workspace lock
  resolves both to 0.1.0, even though abilists 0.1.1 is available. Disabling default
  features removes these defaults; source mode needs `provider-sources`, while
  ABI data without bundling comes from a supplied directory. Feature unification
  can re-enable dependencies through another edge.
- cargo-libc 0.2.0 directly requires `libc-abi-tools`, `kernel-abi-tools`,
  `linux-abi-scan`, and `linux-targets`, each with the caret requirement `"0.3.0"`.
  Its lock resolves all four to 0.3.0 and both glibc data crates to 0.1.0.
  It defines no package feature switch disabling these defaults; its
  `libc-abi-tools` dependency enables the provider/data features transitively.
  It does not depend on the `linux-abi-tools` facade.
- Sources embeds the gzip pack and accepted differences. The ABI lists crate
  embeds upstream ABI data. A built consumer uses the selected compiled-in
  versions, not the latest GitHub checkout or a runtime registry refresh.
- `libc-abi-tools` also carries its `libgcc_s` table and pinned container build
  scripts/input manifests. `kernel-abi-tools` embeds syscall and madvise tables;
  `linux-targets` embeds distribution presets. `linux-abi-scan` consumes the
  kernel/target libraries; the `linux-abi-tools` crate re-exports the four libraries.
  Generated provider caches, startup objects and stubs are downstream artifacts,
  not binary objects shipped by this crate. Qualify fresh provider outputs when
  their inputs change; do not assume an existing cache updates itself.

The linux-abi-tools repository publishes **five** packages (all manifests and
workspace lock at that snapshot 0.3.1):

| Package | Internal dependencies | Publish prerequisite |
| --- | --- | --- |
| linux-targets | none | none |
| kernel-abi-tools | linux-targets | linux-targets |
| linux-abi-scan | kernel-abi-tools, linux-targets | both |
| libc-abi-tools | none within workspace | required versions of both glibc data crates |
| linux-abi-tools | all four above | all four |

All internal path dependencies also require version `"0.3.1"`. One valid order is
`linux-targets` → `kernel-abi-tools` → `linux-abi-scan` → `libc-abi-tools` →
`linux-abi-tools`; `libc-abi-tools` can be published earlier once its external data
requirements exist. The existing policy is **lockstep while 0.x**, not selective
package versioning. Do not silently replace it with a selective-release policy.
A provider-only patch does not automatically require any workspace publication;
if a workspace release is chosen, follow its current five-package policy unless
the maintainer explicitly changes that policy.

## Conditional dependency rollout

1. Publish changed data crates first, only if their packaged content needs a
   release. Neither glibc data crate requires a new release of the other merely
   because its peer changed. Coordinate changed release sets/metadata contracts.
2. A compatible sources 0.1.1 satisfies `"0.1.0"`; no downstream manifest edit is
   inherently required. An existing lock remains at 0.1.0 until deliberately
   refreshed. A requirement edit is needed if adopting an incompatible series,
   or intentionally raising the minimum to require a fix/new data.
3. Refresh each affected consumer lock separately and inspect its diff. A local
   lock refresh plus rebuild can consume the new data without publishing a new
   library version. Existing installed binaries do not change when a lock does.
4. Release the ABI workspace only if its own packaged changes or an intentional
   minimum-dependency change justify it. Then follow the dependency order above.
5. Release cargo-libc last only if its code/docs/requirements or distributed lock
   need updating. Its package includes Cargo.lock: distributing a refreshed lock
   for `cargo install --locked` requires a new cargo-libc release. A compatible
   upstream publication alone does not update that installation path.

## Gates for an authorized release

These are future release checks, not results of this documentation-only task.

- Inspect the exact candidate diff against the last published tag, package file
  list and registry contents; identify unchanged versus regenerated data. Keep
  versions, dependency minima, lock resolutions and tag/commit identity coherent.
- Unchanged data: verify no source blobs, release tables, pack, or accepted
  differences changed; no collection job is needed solely for prose edits.
- Regenerated data: never hand-edit `data/`. Use the qualified linux-abi-tools
  work trees and pinned, signature-checked archives; run collector and `--check`,
  re-record `interfaces.tsv` with replay `--record` and `libraries.tsv` with
  `libc-abi glibc libraries`. Preserve origin classes, exact hashes, licenses and
  generated-header provenance. Refer to [DATA.md](DATA.md) and [REPLAY.md](REPLAY.md).
- Regenerate the pack after all table changes, check exact equality to `data/`,
  and rebuild replay so it embeds that pack. Each accepted difference needs its
  exact text and a reason it cannot matter. No unreviewed widening of tolerances.
- Run the [repository validation](../AGENTS.md#validation): fmt, workspace tests,
  clippy with warnings denied; CI uses Rust 1.99 on Windows/Linux and tests MSRV
  1.90. Check documentation with warnings denied and preserve the Rust 1.90 floor.
- Run `cargo package -p glibc-provider-sources --locked`, inspect its file list,
  verify the packaged source build, and enforce CI's 9,000,000-byte ceiling.
  Check notices/license texts and the complete SPDX expression, including UAPI
  exceptions; do not relicense upstream content or add binary distribution.
- Qualify changed replay/compiler behavior or data against the Replay matrix:
  upstream GCC 12–16, Rocky 9 GCC 11, Ubuntu 24.04 GCC 13, Debian 13 GCC 14,
  Fedora 43 GCC 15. Record actual compiler/binutils versions; tags float.
  A documented old run is not evidence that a new candidate passed.
- For changed provider output, coordinate linux-abi-tools' application,
  exhaustive-link and Cargo suites using the candidate data and a pinned
  cargo-libc consumer. Compare source-mode outputs with recorded interfaces;
  also test relevant default/no-default feature combinations. Object replay
  alone is necessary but insufficient for consumer qualification. Documentation
  alone does not justify new target/compiler claims or unrelated regeneration.

Useful explicit commands from the repository root (future execution only):

```sh
cargo run --release --locked --offline -p glibc-provider-collector \
  --bin glibc-provider-pack -- --check
cargo build --release --locked --offline -p glibc-provider-collector \
  --bin glibc-provider-replay
./target/release/glibc-provider-replay \
  --release 2.28 --compiler gcc --out /tmp/replay-candidate-2.28
```

Pack regeneration uses the pack command without `--check` (Linux, `gzip`). Replay
needs GCC/binutils and a fresh output directory. Collector invocation must select
`--bin glibc-provider-collect`; its package has three binaries. Set and quote the
pinned archive hash as `--archive-sha256 "$ARCHIVE_SHA256"`, not shell angle brackets.

## Go/no-go and recovery

- [ ] Maintainer approves concrete scope, semver choice and publication separately.
- [ ] Recheck registry/yank state, tags, concurrent commits and downstream policy.
- [ ] Candidate-specific required checks pass; package, provenance and license
      inventories reviewed; no undocumented compatibility claims.
- [ ] Consumer resolution/rebuild/integration plan matches the changed content;
      any required downstream owners have accepted the rollout.
- [ ] Publish only the reviewed package; preserve its commit/tag identity, then
      verify registry content and versioned docs.rs build status. Record actual
      publication only afterward, not as a speculative changelog entry.

A crates.io version is immutable and cannot be overwritten or reused. Yanking
prevents normal new resolution of that version; it does not remove source already
fetched, undo existing lockfiles, or replace installed binaries/provider caches.
Do not move a released tag. Correct a defective release with a new version and,
when warranted, a separately authorized yank. Rollback for consumers means
selecting a known-good compatible version, reviewing/restoring their lock and
rebuilding/requalifying outputs; a Git revert alone cannot recall a package.

## Documentation cleanup completed (2026-10-31)

Completed in Git, without publishing a crate or regenerating data:

- README now names the published library, dependency, MSRV and x86_64 Linux data
  scope, with a complete non-writing example and focused guide navigation.
- [MAINTAINING.md](MAINTAINING.md) documents collect/check, interface/library
  recording, safe hash handling, pack generation, rebuild and replay sequencing.
- [DATA.md](DATA.md#using-the-library) covers extraction and fresh directories;
  [REPLAY.md](REPLAY.md) covers build prerequisites, reports and concurrency.
- README retains concise provenance/license information and links compiler claims
  to dated qualification evidence. Detailed reference material stays in guides.
- The manifest's authority comment points to AGENTS.md. Its package settings,
  dependencies and versions are unchanged. The seven-table description is correct.

Remaining gates are the release checklist above, including publication approval,
package/MSRV/license checks and any consumer qualification warranted by a future
code/data change. The updated packaged README does not replace the README in
already-published 0.1.0; publication is a separate decision. Recheck the recorded
crates.io page-rendering discrepancy before any release.
