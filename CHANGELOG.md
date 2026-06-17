# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.1] - 2026-06-17

### Fixed

- **Commit hook now uses the configured API key**: the default generation path only
  attempted the OpenAI multi-step request when the `OPENAI_API_KEY` environment variable
  was set, silently ignoring a key stored via `git ai config set openai-api-key`. As a
  result every commit fell through to the local programmatic generator and emitted
  filename-derived subjects like `Update <file>`. The client is now built from the stored
  configuration (honoring a custom `openai-base-url` too), falling back to the environment
  variable only when the config holds no usable key.
- **Local-fallback truncation no longer splits words**: oversized fallback subjects are
  trimmed to the last whole word (no more `Update controlle`) instead of a hard mid-word cut.

## [1.2.0] - 2026-06-17

### Added

- **Configurable OpenAI model**: any model string is now accepted (not just the built-in set)
  and is verified to exist (via the OpenAI/compatible `models` API) at `git ai config set model`
  time — best-effort, so offline users aren't blocked. Unknown models fall back to a safe
  tokenizer/context size.
- **Custom OpenAI base URL** (`git ai config set openai-base-url <url>`) — point git-ai at a
  local or self-hosted OpenAI-compatible endpoint (e.g. ollama).

### Changed

- **Default model is now `gpt-4.1-mini`** (was `gpt-4.1`) — near-equivalent quality for short,
  structured commit messages at lower latency and cost on a per-commit tool.
- **Faster large diffs**: truncation now tokenizes once and decodes a token prefix instead of
  re-tokenizing on every binary-search step; parallel diff processing reworked to be both
  deterministic and correct.
- **Deterministic commit messages**: identical staged changes now always produce the same patch
  (and thus message) — previously affected by `HashMap` iteration order.
- Tightened the commit-message prompts (imperative mood, no trailing period, impact-first,
  anti-hallucination), with invariant tests.
- Upgraded all dependencies including async-openai 0.41, git2 0.21, tiktoken-rs 0.12, reqwest 0.13;
  pinned the Rust nightly toolchain for reproducible builds.
- Removed dead code (orphaned `ollama`/`client`/`profile` modules and the unused `model::run`).
- Honor the configured `max-commit-length` in the final message selection (was hardcoded to 72).

### Security

- Added `cargo audit` and qlty to CI; hardened all GitHub workflows (least-privilege permissions,
  SHA-pinned actions, fixed script-injection) and added datajust-style PR governance workflows.

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
