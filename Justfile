set shell := ["bash", "-cu"]

GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"
# GITHUB_TAG := `shell git describe --tags --abbrev=0`

github-actions:
    act --container-architecture linux/amd64
install:
  cargo install --path .
  git ai hook uninstall || true
  git ai hook install
test:
  cargo test --all
build_hook:
  cargo build --bin hook --release
install_hook: build_hook
  gln -rfs target/release/hook .git/hooks/prepare-commit-msg
simulate:
  ./simulate.sh
release:
  #!/usr/bin/env bash
  version=$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].version' | tr -d '\n')
  echo "Releasing $version"
  git tag -a v$version -m "Release v$version"
  git push origin v$version
  git push origin main
  git push --tags
act:
  act --container-architecture linux/amd64
