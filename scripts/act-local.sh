#!/bin/bash
# Helper script to run specific matrix combinations with act

set -e

# Default to Linux GNU target
TARGET="${1:-linux-gnu}"

case "$TARGET" in
  "linux-gnu")
    echo "Running Linux GNU build..."
    act -j artifact -W .github/workflows/cd.yml \
      --matrix os:ubuntu-latest \
      --matrix target:x86_64-unknown-linux-gnu
    ;;
  "linux-musl")
    echo "Running Linux musl build (will be skipped due to cross-compilation issues)..."
    act -j artifact -W .github/workflows/cd.yml \
      --matrix os:ubuntu-latest \
      --matrix target:x86_64-unknown-linux-musl
    ;;
  "macos-x86")
    echo "Running macOS x86_64 build (will be skipped in act)..."
    act -j artifact -W .github/workflows/cd.yml \
      --matrix os:macos-latest \
      --matrix target:x86_64-apple-darwin
    ;;
  "macos-arm")
    echo "Running macOS ARM64 build (will be skipped in act)..."
    act -j artifact -W .github/workflows/cd.yml \
      --matrix os:macos-latest \
      --matrix target:aarch64-apple-darwin
    ;;
  "all")
    echo "Running all builds (macOS targets will be skipped)..."
    act -j artifact -W .github/workflows/cd.yml
    ;;
  *)
    echo "Usage: $0 [linux-gnu|linux-musl|macos-x86|macos-arm|all]"
    echo ""
    echo "Available targets:"
    echo "  linux-gnu   - Build for x86_64-unknown-linux-gnu (works in act)"
    echo "  linux-musl  - Build for x86_64-unknown-linux-musl (skipped in act)"
    echo "  macos-x86   - Build for x86_64-apple-darwin (skipped in act)"
    echo "  macos-arm   - Build for aarch64-apple-darwin (skipped in act)"
    echo "  all         - Run all builds (only Linux GNU will actually build)"
    exit 1
    ;;
esac
