# syntax=docker/dockerfile:1
# TODO Reviewed ??-??-2026

ARG RUST_IMAGE_LABEL=1.99.1-trixie
FROM rust:${RUST_IMAGE_LABEL} AS development

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt \
    && rustup toolchain install nightly-2026-10-22 --profile minimal --component rustfmt

ARG CARGO_NEXTEST_VERSION=0.9.145
ARG PREK_VERSION=0.5.6
ARG DPRINT_VERSION=0.64.1
ARG CARGO_DENY_VERSION=0.20.2
ARG JUST_VERSION=1.59.0

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall --no-confirm --secure --version "${CARGO_NEXTEST_VERSION}" cargo-nextest \
    && cargo binstall --no-confirm --secure --version "${PREK_VERSION}" prek \
    && cargo binstall --no-confirm --secure --version "${DPRINT_VERSION}" dprint \
    && cargo binstall --no-confirm --secure --version "${CARGO_DENY_VERSION}" cargo-deny \
    && cargo binstall --no-confirm --secure --version "${JUST_VERSION}" just

WORKDIR /code

CMD ["sleep", "infinity"]