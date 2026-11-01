# syntax=docker/dockerfile:1
# TODO Reviewed ??-??-2026

ARG RUST_IMAGE_LABEL
FROM rust:${RUST_IMAGE_LABEL} AS development

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
    && rm -rf /var/lib/apt/lists/*

ARG RUSTFMT_TOOLCHAIN
COPY rustfmt-toolchain.toml /usr/local/share/rustfmt-toolchain.toml
RUN rustup component add clippy rustfmt \
    && RUSTFMT_TOOLCHAIN="${RUSTFMT_TOOLCHAIN:-$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' /usr/local/share/rustfmt-toolchain.toml)}" \
    && rustup toolchain install "$RUSTFMT_TOOLCHAIN" --profile minimal --component rustfmt
ENV RUSTFMT_TOOLCHAIN=${RUSTFMT_TOOLCHAIN}

ARG CARGO_NEXTEST_VERSION
ARG PREK_VERSION
ARG DPRINT_VERSION
ARG CARGO_DENY_VERSION
ARG JUST_VERSION
ARG SCCACHE_VERSION=0.19.0

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall --no-confirm --secure --version "${CARGO_NEXTEST_VERSION}" cargo-nextest \
    && cargo binstall --no-confirm --secure --version "${PREK_VERSION}" prek \
    && cargo binstall --no-confirm --secure --version "${DPRINT_VERSION}" dprint \
    && cargo binstall --no-confirm --secure --version "${CARGO_DENY_VERSION}" cargo-deny \
    && cargo binstall --no-confirm --secure --version "${JUST_VERSION}" just \
    && cargo binstall --no-confirm --secure --version "${SCCACHE_VERSION}" sccache

WORKDIR /code

CMD ["sleep", "infinity"]
