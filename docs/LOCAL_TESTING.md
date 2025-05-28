# Local Testing with Act

## Overview

This document explains how to test GitHub Actions workflows locally using `act`.

## Running Specific Matrix Combinations

### Option 1: Using act filters (Recommended)

To run only specific matrix combinations locally, use act's job filtering:

```bash
# Run only the macOS x86_64 build
act -j artifact -W .github/workflows/cd.yml --matrix os:macos-latest --matrix target:x86_64-apple-darwin

# Run only the Linux GNU build
act -j artifact -W .github/workflows/cd.yml --matrix os:ubuntu-latest --matrix target:x86_64-unknown-linux-gnu
```

### Option 2: Create a custom event file

Create a file `.act.json` with specific matrix values:

```json
{
  "workflow_dispatch": {
    "inputs": {
      "matrix_filter": "macos"
    }
  }
}
```

Then run:
```bash
act workflow_dispatch -e .act.json
```

## Known Issues

### Cross-compilation for musl targets

When running workflows locally with `act` on macOS, cross-compilation for `x86_64-unknown-linux-musl` targets will fail. This is because:

1. `act` runs Linux containers on macOS
2. The build process tries to compile OpenSSL from source for musl
3. The OpenSSL build system incorrectly uses `-m64` flag with musl-gcc, which doesn't support it

### Cross-compilation for macOS targets

When running in `act` (Linux containers), you cannot build macOS targets (`x86_64-apple-darwin`, `aarch64-apple-darwin`) because:

1. `act` runs in Linux containers
2. Cross-compiling from Linux to macOS requires Apple's SDK and toolchain
3. The compiler flags like `-arch` and `-mmacosx-version-min` are not recognized by Linux gcc

### Solutions

#### 1. Run only Linux targets locally

The safest approach is to only test Linux targets locally:

```bash
# Test Linux GNU target
act -j artifact -W .github/workflows/cd.yml --matrix os:ubuntu-latest --matrix target:x86_64-unknown-linux-gnu
```

#### 2. Skip problematic builds

The main workflow automatically detects when running in `act` and skips musl builds. It will create placeholder binaries instead.

#### 3. Test on real GitHub Actions

Push your changes to a branch and let the real GitHub Actions runners handle the cross-compilation:

```bash
git push origin feature/your-branch
```

## Configuration

The `.actrc` file is configured to:
- Use appropriate container images
- Set up caching
- Enable the `ACT` environment variable for detection

## Troubleshooting

If you encounter issues:

1. Ensure Docker is running
2. Update act to the latest version: `brew upgrade act`
3. Clear the cache: `rm -rf tmp/cache`
4. Check the `.actrc` configuration
5. Use `-v` flag for verbose output: `act -v`
