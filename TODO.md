# TODO

## Fixes
- [ ] Implement `process_all_directives` scope configuration in `directive_lib`.
- [ ] Fix `convert_links` implementation in `inline_links` (currently a stub).
- [ ] Ensure unique temporary file names in `core_lib::process_text_file` to support parallel processing.
- [ ] Handle empty updated content correctly in `core_lib::process_text_file`.

## Improvements
- [ ] Add support for nested directives in `parse_lib`.
- [ ] Use memory mapping for large files in `core_lib::process_text_file`.
- [ ] Implement parallel directory walking and processing in `core_lib::walk_directory_and_process_files` (e.g., using `Rayon`).
- [ ] Support GFM-style tables and task lists in `parse_lib`.
- [ ] Complete `rust_rate` implementation.
- [ ] Remove `#![allow(unused)]` from `parse_lib` after stabilizing the API.

## Documentation & Testing
- [ ] Add more comprehensive integration tests for all CLI tools.
- [ ] Improve API documentation with more examples.
- [ ] Set up CI/CD for automatic testing and linting.
