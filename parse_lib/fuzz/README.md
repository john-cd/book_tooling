<!-- TODO Reviewed ??-??-2026. -->

# Fuzzing `parse_lib`

This crate is excluded from the main workspace. It needs nightly Rust and
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz), and is not built in CI.

```bash
cargo install cargo-fuzz
cd parse_lib
cargo +nightly-2026-11-16 fuzz list
cargo +nightly-2026-11-16 fuzz run round_trip -- -max_total_time=60
```

Targets live in `fuzz_targets/`:

- `round_trip`: `parse_document` then `MarkdownRenderer`; asserts the rendered text equals the input.

Crashing inputs are saved under `fuzz/artifacts/`; reproduce one with
`cargo +nightly-2026-11-16 fuzz run round_trip fuzz/artifacts/round_trip/<file>`.
