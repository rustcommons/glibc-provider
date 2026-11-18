# Digest-pinned OSS-Fuzz builder: Rust nightly-2025-09-05 and cargo-fuzz are
# preinstalled, so the build downloads no installer. Update the digest by hand.
FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:3428d0a90b1e7efbe2ba947bd1d0c9a70594986b4b6553804547f1fb24cfc92f
ENV RUSTUP_TOOLCHAIN=nightly-2025-09-05
COPY . $SRC/glibc-provider
WORKDIR $SRC/glibc-provider
COPY .clusterfuzzlite/build.sh $SRC/
