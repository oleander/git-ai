# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.2] - 2026-06-16

### Fixed

- **Multi-step pipeline broken since v1.1.1**: `collect_diff_data()` dropped git
  file/hunk header lines (libgit2 origins `'F'`/`'H'`), so the generated patch had no
  `diff --git`/`@@` markers. `parse_diff()` could then no longer split the diff into
  per-file sections and collapsed every commit into a single `"unknown"` file, producing
  generic commit messages. Headers are now preserved. (#91)
- Annotated an `f32` literal in `model::run()` so the crate builds clean under the latest
  nightly (the new `float_literal_f32_fallback` lint).

### Tests

- Added real-git2 regression tests (`tests/patch_test.rs`) asserting headers are preserved
  and that `to_patch()` output parses back into the correct per-file sections (never
  `"unknown"`).
