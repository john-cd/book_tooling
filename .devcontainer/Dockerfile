# syntax=docker/dockerfile:1
# Dockerfile build inputs are required and supplied by Compose from tracked .devcontainer/.env.
# From the repo root pass --env-file .devcontainer/.env and add --env-file .devcontainer/.env.local when present.
# Dockerfile build inputs for book-tooling.
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
RUN rustup component add clippy rustfmt \
    && rustup toolchain install "$RUSTFMT_TOOLCHAIN" --profile minimal --component rustfmt
ENV RUSTFMT_TOOLCHAIN=${RUSTFMT_TOOLCHAIN}

ARG CARGO_NEXTEST_VERSION
ARG CARGO_BINSTALL_VERSION
ARG PREK_VERSION
ARG DPRINT_VERSION
ARG CARGO_DENY_VERSION
ARG JUST_VERSION
ARG SCCACHE_VERSION

RUN cargo install --locked --version "${CARGO_BINSTALL_VERSION}" cargo-binstall \
    && cargo binstall --no-confirm --secure --version "${CARGO_NEXTEST_VERSION}" cargo-nextest \
    && cargo binstall --no-confirm --secure --version "${PREK_VERSION}" prek \
    && cargo binstall --no-confirm --secure --version "${DPRINT_VERSION}" dprint \
    && cargo binstall --no-confirm --secure --version "${CARGO_DENY_VERSION}" cargo-deny \
    && cargo binstall --no-confirm --secure --version "${JUST_VERSION}" just \
    && cargo binstall --no-confirm --secure --version "${SCCACHE_VERSION}" sccache

WORKDIR /code

CMD ["sleep", "infinity"]
