# TODO

- [ ] P1 remove remaining panic-on-error paths from reusable runtime code
- [ ] P1 `book_tool::links::AsyncLinkChecker::new`, `book_tool::links::convert`, and `rust_rate::crawler::GithubCrawler::new` still rely on `unwrap`/`expect`; return errors or encode the invariants without panicking.
- [ ] P1 make `link-check --delete` require `--refdefs`. the CLI currently accepts `--delete` without `--refdefs` and then silently skips refdef cleanup; tighten clap validation and add a regression test.
- [ ] P1 make directive config loading failures observable
- [ ] P1 `directives::execute` falls back to defaults for malformed or unreadable `config.toml`; log the real error. invalid config should fail fast.
- [ ] P1 standardize per-file parse failure policy across `book_tool` commands
- [ ] P1 `inline_links` logs parse failures as errors while `directives`, `search`, `extract_comments`, and `link_check` warn and continue; decide the contract and encode it in shared helpers/tests.
- [ ] P1 expand link-check retry regression coverage
- [ ] P1 add workflow coverage for `503`, invalid or missing `Retry-After`, and max-retry exhaustion in `AsyncLinkChecker`.
- [ ] P2 add refdef deduplication regression coverage in `parse_lib`
- [ ] P2 `parse_lib::visit::refdefs::get_sorted_dedupe_refdefs` has no direct coverage for duplicate-label and duplicate-URL edge cases.
- [ ] P1 add a lightweight repo hygiene check for orphan source files
- [ ] P1 refactors have left uncompiled `src/**/*.rs` files behind before; add a check or documented review step so the filesystem and module tree stay aligned.

## LATER

- [ ] revisit branch strategy once the refactor backlog is smaller
