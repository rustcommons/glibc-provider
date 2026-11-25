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
   source. Before Rust setup or any third-party action, it signs `SHA256SUMS`
   with the `crates-io` environment secrets `RELEASE_SIGNING_KEY` and
   `RELEASE_SIGNING_PASSPHRASE`, then verifies in a separate temporary GnuPG home
   against the committed public key and pinned primary fingerprint. Both homes
   and their agents are removed on success or failure. It then uses the runner's
   own `rustup` to select Rust 1.99.0 and repackages each crate without building
   (`cargo package --locked --no-verify`), requiring byte-identical (`cmp`)
   output to the attested file. Signing happens even when every crate is already
   published, and a failure stops publication. Only then does it request a
   short-lived Trusted Publishing token, available only to the step that runs `cargo publish --locked
   --no-verify`. A version crates.io already has is skipped, so a re-run is safe.
5. **GitHub release (automatic).** The pipeline creates release `vX.Y.Z` from
   `docs/releases/vX.Y.Z.md` as a draft and attaches the `.crate`, the SBOM,
   `SHA256SUMS`, `SHA256SUMS.asc` and the provenance bundle. It checks metadata
   and each asset's name, size and downloaded digest before publishing and
   checks again afterwards. The maintainer no longer creates the release by
   hand. A re-run resumes a draft and uploads only missing files; it validates
   an existing published release without modifying it. Mismatches fail. A
   previously uploaded signature is retained if it verifies the identical
   checksums with the pinned key, since reruns change signature timestamps.
6. **Verify.** The pipeline verifies the downloaded checksum signature in an
   isolated home against the pinned primary fingerprint and checks every listed
   asset with `sha256sum --check`. It also checks that the live release body
   equals the notes,
   that crates.io's checksum and the official download equal the attested
   `.crate`, the attestations with `gh attestation verify`, and the docs.rs
   build (still pending after 20 minutes is reported as pending, not a pass).
7. **Record PR.** Record the tag, run, checksums, docs.rs build and any manual
   checks in the [release plan](RELEASE-PLAN.md) in a docs-only PR.

Manual runs from the Actions tab and PRs that touch the workflow, its scripts,
`CHANGELOG.md` or `docs/releases/` are dry runs: gate and build only. PRs from
forks do not run this workflow's jobs. Build runs the signing tests with an
ephemeral passphrase-protected key; production commands accept no key or
fingerprint override. No real signing secrets are available to the dry run.

## Check the gate locally

The gate is stdlib Python and needs `cargo`. From a checkout of the tag:

```sh
GH_TOKEN=$(gh auth token) python3 scripts/release-gate.py \
  --event push --ref-name vX.Y.Z --sha "$(git rev-parse 'vX.Y.Z^{commit}')" \
  --repo rustcommons/glibc-provider
```

It prints every failed check and exits 1 when a tag run would be refused.

## Verify downloaded release assets

Releases after 0.1.2 include `SHA256SUMS.asc`; historical releases are unchanged.
Download every file listed in `SHA256SUMS`, plus `SHA256SUMS.asc`, from the same
GitHub Release. Obtain the [committed public key](../.github/release-signing-key.asc)
from the reviewed repository and check its primary fingerprint against
`B281C3010A76A11737B4D066C7396F6EA7647CC6` before trusting it:

```sh
export GNUPGHOME=$(mktemp -d)
chmod 700 "$GNUPGHOME"
gpg --import release-signing-key.asc
gpg --with-colons --fingerprint
# Require primary fingerprint B281C3010A76A11737B4D066C7396F6EA7647CC6 above.
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check SHA256SUMS
gpgconf --kill gpg-agent
rm -rf "$GNUPGHOME"
unset GNUPGHOME
```

Stop if any command fails. A valid signature from another primary key is not
accepted. From a reviewed checkout, `python3 scripts/release_signing.py verify
/path/to/SHA256SUMS` automates the fingerprint, signature and checksum checks.
The signing subkey is `248DFD639B75D1EFF2A87289D7C7DE7BBCCF35EC`; the verifier pins
the primary identity so an owner-authorized subkey rotation can retain it.
Revoked keys and bad, expired-signature or unknown-key statuses are refused.
Verification requires exactly one good signature and one `VALIDSIG` for the
pinned primary. An expired signing subkey is accepted only if the signature's
creation timestamp precedes that subkey's expiry in the committed key, so old
releases remain verifiable after routine rotation.
