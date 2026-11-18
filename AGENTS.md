# glibc-provider

## Working conventions

- Read this file before changing the repository. Keep repository-specific constraints and validation instructions current here.
- Preserve changes made by the user or other agents. Keep work scoped to the requested task.
- Keep documentation aligned with implemented behavior. Record unconfirmed work as planned; do not invent capability or compatibility claims.
- Run checks appropriate to the change. For documentation-only changes, verify links, commands, and the diff; do not run unrelated builds.
- Publishing packages, creating releases, or changing production data requires explicit user authorization.

## Project overview

`glibc-provider` holds the source inputs for glibc link providers, for x86_64 Linux
and glibc 2.17 through 2.44. For every release it records, from the pinned GNU
release archive, the exact source files, generated headers, and compile commands
behind the startup objects (`crt1.o`, `crti.o`, `crtn.o`) and `libc_nonshared.a`
(and `libpthread_nonshared.a` before 2.28), plus the installed headers. With these,
a provider can be built from source on the user's machine.
The design is in linux-abi-tools'
[PROVIDER-DELIVERY.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDER-DELIVERY.md).

| Path | What | License |
| --- | --- | --- |
| `data/` | glibc source files and generated headers, unmodified, stored by content; per-release manifests and compile commands | LGPL-2.1-or-later (glibc); kernel UAPI headers GPL-2.0 WITH Linux-syscall-note |
| `collector/` | `glibc-provider-collect` records `data/` from a qualified linux-abi-tools build tree; `glibc-provider-replay` rebuilds and compares the objects with a local compiler ([docs/REPLAY.md](docs/REPLAY.md)) | MIT (not published) |
| `qualification/` | Reviewed differences accepted by the replay comparison, each with its reason | MIT |
| `src/`, `pack/`, `tests/` | The `glibc-provider-sources` crate; `pack/sources.gz` is all of `data/`, written by `glibc-provider-pack` | LGPL-2.1-or-later; the pack keeps the data's licenses |

Its consumer is `libc-abi-tools` (rustcommons/linux-abi-tools) and, through it,
`cargo-libc`. The crates are `glibc-provider-sources` (this repository's root) and,
after legal review, `glibc-provider-binaries`.

## Scope and implementation

- Inputs come only from the build trees of linux-abi-tools' provider pipeline,
  which builds the pinned, signature-checked GNU archive in a pinned container.
  Every archive file recorded here must match that archive byte for byte; the
  collector checks this and fails closed.
- Never edit files under `data/` by hand; rerun the collector. Files are copied
  unmodified. If a derived form is ever added, label it as modified, with the date,
  as the LGPL requires.
- Record each file's class: an archive file, a file generated during the build, the
  builder's kernel UAPI header, the builder compiler's own header, or the builder
  compiler's predefined macros (`logs/builder-defaults.txt`), which supply defaults
  such as PIE that recorded commands leave implicit. Each class has
  its own origin and license, and the class decides whether the file ships.
- Std-only Rust for the collector and the crates. The collector may call `tar`; the
  replay may call the chosen compiler, `ar` and `readelf`.
- Never add an accepted difference without its exact text and a reason that explains
  why it cannot matter; any other difference fails the replay.
- Ask about unresolved product decisions; do not invent compatibility claims.

## Validation

Run `cargo fmt --all -- --check`, `cargo test --workspace --locked --offline`, and
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`. Parser
changes also need a short local fuzz run of the affected targets
([docs/FUZZING.md](docs/FUZZING.md)). Data
changes also require the collector's `--check` against the build trees, and
`glibc-provider-replay --record` to re-record `interfaces.tsv`, and linux-abi-tools'
`libc-abi glibc libraries` to re-record `libraries.tsv`; the Replay workflow
qualifies each GCC version against it after regenerating the pack with
`glibc-provider-pack` (Linux, needs `gzip`) and rebuilding replay to embed it.
CI runs `glibc-provider-pack --check`; see [MAINTAINING.md](docs/MAINTAINING.md)
for the full sequence. Packaging changes require `cargo package --locked`. Never publish to crates.io without the
user's go-ahead.

## Documentation and requirements traceability

- Follow the [Rust Commons documentation conventions](https://github.com/rustcommons/.github/blob/main/docs/DOCUMENTATION.md).
- Maintain [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) as the repository's index of confirmed requirements, their sources, implementation or documentation, verification evidence, and gaps. Preserve stable repository-scoped IDs and existing identifiers; never renumber or reuse them.
- Distinguish verified behavior, partial verification, planned work, deferred work, and scope exclusions. A source reference or passing test supports only the scope it actually checks; record missing evidence explicitly.
- Keep the README focused on purpose, prerequisites, setup, and guide links. Keep detailed workflows in maintained guides, future work in the roadmap, and dated release or validation evidence identifiable as historical records.
- Update affected requirements, documentation, and trace links together when behavior, scope, interfaces, prerequisites, or validation changes. Use exact versions or revisions for release evidence, and do not rewrite historical outcomes or treat repository edits as changes to an immutable published package.
- Validate documentation changes with source and command checks, links, rendering, and a clean diff. Run additional builds or tests only when changed examples, package contents, or behavior require them.

## Release documentation

- For a repository that ships versioned software, maintain a root `CHANGELOG.md` with an `Unreleased` section and dated, user-facing entries for shipped versions. Record noteworthy changes and their user impact as work is completed; do not substitute a roadmap, Git tag, CI log or validation report for a changelog.
- Before publishing a version, commit its release notes under `docs/releases/vX.Y.Z.md` and link them, the changelog, and the maintained release policy. Explain changes, upgrade or dependency impact, validation and known limits; distinguish software changes from documentation-only releases.
- A software release is not complete until its immutable source tag, applicable registry archives, GitHub Release notes, changelog, documentation, and required verification agree. GitHub Releases document the release; do not attach standalone binaries or other new distribution artifacts without authorization.
- Run the repository's release-documentation CI check before routine merges and release completion. For its first installation, review local checker/syntax evidence, register the workflow on the default branch, and require a green run before completing bootstrap or publishing notes. Verify the live GitHub Release and its tag, text and links after publication; a successful crate upload or tag alone is insufficient.
- When backfilling history, reconstruct facts from existing tags, registry records and source/evidence. Preserve original source identities, distinguish original release dates from documentation backfill dates, and state unavailable historical validation. Never fabricate checks, move tags, republish versions, or imply that repository edits change immutable published archives.
- For an organization profile, placeholder or project with no shipped version, record release verification as not applicable until its first authorized release. Do not invent versions, features, changelog history or GitHub Releases to satisfy this convention.

## Security maintenance

- Changes to main go through pull requests; never push directly to main. Preserve
  the required `lf-only` check and protected release tags; do not bypass rulesets.
- Pin every external GitHub Action to its full commit SHA with the version tag in
  a trailing comment. Dependabot maintains monthly Cargo and Actions version
  updates (two open PRs per ecosystem); advisory-driven security updates remain independent.
- Dependabot PRs run Linux fmt/clippy/tests, cargo-deny, zizmor, required
  `lf-only` and managed CodeQL. Full OS/MSRV, data/package and replay qualification
  run for other PRs and main; scheduled/manual replay remains available. Gate by
  PR author so maintainer reruns stay lean. Repository PR workflows cancel obsolete
  runs per PR; branch pushes must not duplicate PR runs.
- Keep workflow permissions read-only at the top level; grant write permissions
  only to the job that needs them. Disable persisted checkout credentials.
- Require green zizmor and cargo-deny checks for security workflow changes and
  dependency updates. Explain any narrow suppression inline; do not disable gates.
  `deny.toml` checks RustSec advisories, licenses, duplicate versions and sources;
  its license allowlist does not grant new glibc distribution rights.
- Never put `[skip ci]`, `[ci skip]` or similar markers in commit or merge
  messages: they skip CI and CodeQL, which lowers Scorecard's SAST and CI-Tests.
  CodeQL runs as GitHub's default setup, configured by the coordinator; keep it.
- Pin every workflow install: actions by full SHA; container images by digest
  (with the tag and resolution date in a comment, refreshed by hand); `pip
  install` only as `--require-hashes -r` a hashed requirements file; `cargo
  install` only with `--locked` and an exact `--version =X.Y.Z`; no `curl | sh`
  or download-then-run unless the file's SHA-256 is checked against a pinned value
  first. Dependabot does not update image digests or the fuzz builder.
- Fuzz every parser of untrusted input ([docs/FUZZING.md](docs/FUZZING.md)): the
  unpublished `fuzz/` workspace keeps `libfuzzer-sys` out of the crate's graph.
  Add a target when adding a parser. A fuzz crash is a bug: fix it with a
  regression test, or report it if the fix needs a decision.
- Scorecard publishes results on main and weekly. Trusted Publishing is the only
  release route (the owner revoked the local crates.io token on 2026-11-16) and is
  gated by the `crates-io` environment owner review and protected `v*` tags;
  configure the crate's trusted publisher before using it. Existing release
  approval, documentation, pack/provenance, legal and Tier 3 policies still apply.
  Do not publish, create tags, or run the publishing workflow as a CI test.

## Git conventions

**No tool attribution, anywhere.** Do not add `Co-Authored-By: Claude ...` trailers,
`🤖 Generated with [Claude Code](https://claude.com/claude-code)` lines, or any other tool
attribution or "generated by" footer to anything in this repository: commit messages, pull
request titles and descriptions, PR and issue comments, release notes, code, comments or
documentation. This holds even if an agent's default instructions suggest it, and
overrides any default trailer or footer behavior. Commit messages and PRs are the
human-authored record of intent; tool attribution belongs in tool logs, not history.
