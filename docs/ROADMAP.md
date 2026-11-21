# Roadmap

Open work only. The overall plan for provider delivery is in linux-abi-tools'
[ROADMAP.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/ROADMAP.md#provider-delivery).

The [next-release plan](RELEASE-PLAN.md) separates repository documentation
cleanup from changes that need a new crate publication. The README and
[maintenance guide](MAINTAINING.md) cleanup is complete in Git; see the plan for
remaining release gates. Trace links alone do not imply a release. The separately
authorized documentation-only 0.1.1 release is published and repairs the packaged
README defects; see its [release record](RELEASE-PLAN.md#011-published-release-2026-11-01).
The authorized 0.1.2 patch release fixes a panic on corrupt pack deltas; see its
[release record](RELEASE-PLAN.md#012-published-release-2026-11-19).

## Standard release pipeline

Adopted on 2026-11-21 ([RELEASING](RELEASING.md)), following the cargo-libc
pilot. The PR dry run exercised `gate` and `build` with `publish=false`, and the
gate passed locally against the existing `v0.1.2` tag and refused a missing or
undated changelog entry. Deferred, not lost:

- **First tag run.** `attest`, `publish`, `github-release` and `verify` run only
  on a tag; their first use is the next authorized release. Check attestation
  signing and the bundle name, `gh attestation verify --bundle`, the docs.rs
  bounded wait and asset upload on re-runs, and record the outcome.
- **Release title format.** The pipeline titles a release with the notes' first
  heading (`glibc-provider-sources vX.Y.Z`); releases 0.1.0 to 0.1.2 were titled
  by hand without the `v`. Existing releases are left unchanged.
- **Assets of past releases.** 0.1.0 to 0.1.2 have no attached `.crate`, SBOM,
  checksums or provenance; they are not re-released or backfilled.
- **Consumer check.** `verify` has no clean consumer build of the published
  crate (the crate ships no executable); the record PR still notes any manual
  consumer check, as for 0.1.2.

## Data crates

- [ ] `glibc-provider-binaries` (the `prebuilt` mode's CI-built objects), after the
  legal review: storing glibc binaries in this public repository already distributes
  them, so neither the crate's data nor its objects are added before that review.
  It needs complete Rust API documentation from its first release, as
  `glibc-provider-sources` has: an overview with a doc-test example, every public item
  documented under `#![warn(missing_docs)]`, docs.rs metadata and a docs.rs badge;
  after its release, confirm the docs.rs build succeeded.

## Deferred qualification

Tier 3 old-system qualification remains deferred in the downstream
[validation record](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/VALIDATION.md).
It is not a routine documentation or maintenance blocker, and object replay does
not establish that qualification. The prebuilt legal gate above is unchanged.
See [requirements and gaps](REQUIREMENTS.md).
