# Releasing glibc-provider-sources

Since 2026-11-21 this repository uses the Rust Commons
[standard release pipeline](https://github.com/rustcommons/.github/blob/main/docs/TRUSTED-PUBLISHING.md)
in [trusted-publishing.yml](../.github/workflows/trusted-publishing.yml). It
publishes `glibc-provider-sources` only; the collector and the fuzz workspace are
never published. Release gates, provenance, legal and Tier 3 policies are in the
[release plan](RELEASE-PLAN.md), and the publication records of past versions
stay there.

A release needs the owner's authorization. A tag or a green check alone is not
authorization.

## Flow

1. **Release PR.** Bump the version in `Cargo.toml` and `Cargo.lock` (and
   `fuzz/Cargo.lock`, whose `--locked` build fails otherwise). Move the
   Unreleased entries of `CHANGELOG.md` under a dated `## [X.Y.Z] - YYYY-MM-DD`
   heading, and write `docs/releases/vX.Y.Z.md` (Changes, Upgrade, Validation
   and limits, Links; absolute links only). These notes are final before
   tagging: the pipeline publishes them unchanged as the GitHub Release body,
   titled with their first heading. The PR's dry run of the pipeline runs the
   gate and build with `publish=false`.
2. **Tag.** After the coordinator merges the PR and CI is green on that merge
   commit on `main`, create an annotated tag `vX.Y.Z` on exactly that commit
   and push it. Never move or re-push a tag.
3. **Release gate, build, attestations.** The tag starts the workflow. The gate
   refuses the run unless the tag is annotated and matches the crate version,
   the commit is on `main` with a successful CI run, and the changelog has the
   dated entry and the notes exist. Build packages the crate (building it from
   its packaged source), checks the 9,000,000-byte size limit, writes a
   CycloneDX SBOM and `SHA256SUMS`. Attestations sign SLSA build provenance for
   the `.crate` and the SBOM, saved as `glibc-provider-X.Y.Z.crates.intoto.jsonl`.
4. **Owner approval.** `Publish to crates.io` waits for the owner's approval of
   the `crates-io` environment. Nothing compiles in this job, because it holds
   `id-token` and the environment; the build job already built the packaged
   source. It repackages each crate without building (`cargo package --locked
   --no-verify`) and requires it to be byte-identical (`cmp`) to the attested
   file. Only then does it request a short-lived Trusted Publishing token,
   available only to the step that runs `cargo publish --locked --no-verify`. A version crates.io already has is skipped,
   so a re-run is safe.
5. **GitHub release (automatic).** The pipeline creates release `vX.Y.Z` from
   `docs/releases/vX.Y.Z.md` and attaches the `.crate`, the SBOM, `SHA256SUMS`
   and the provenance bundle. The maintainer no longer creates the release by
   hand. A re-run uploads only missing files.
6. **Verify.** The pipeline checks that the live release body equals the notes,
   that crates.io's checksum and the official download equal the attested
   `.crate`, the attestations with `gh attestation verify`, and the docs.rs
   build (still pending after 20 minutes is reported as pending, not a pass).
7. **Record PR.** Record the tag, run, checksums, docs.rs build and any manual
   checks in the [release plan](RELEASE-PLAN.md) in a docs-only PR.

Manual runs from the Actions tab and PRs that touch the workflow, its scripts,
`CHANGELOG.md` or `docs/releases/` are dry runs: gate and build only. PRs from
forks do not run this workflow's jobs.

## Check the gate locally

The gate is stdlib Python and needs `cargo`. From a checkout of the tag:

```sh
GH_TOKEN=$(gh auth token) python3 scripts/release-gate.py \
  --event push --ref-name vX.Y.Z --sha "$(git rev-parse 'vX.Y.Z^{commit}')" \
  --repo rustcommons/glibc-provider
```

It prints every failed check and exits 1 when a tag run would be refused.
