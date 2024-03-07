set shell := ["bash", "-cu"]

GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"
LOCAL_IMG := "git-ai:latest"
RUST_IMG := "rust:1.76.0"

local-github-actions:
    act --container-architecture linux/amd64

build-hook: (docker-run RUST_IMG "cargo build --bin hook")

# release:
#     $(docker-cmd) bash -c "\
#     cargo update && \
#     git add Cargo.lock Cargo.toml && \
#     git commit --no-edit && \
#     version=$$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].version' | tr -d '\n') && \
#     echo 'Releasing $$version' && \
#     git tag -a v$$version -m 'Release v$$version' && \
#     git push origin v$$version && \
#     git push origin main && \
#     git push --tags"

local-install-hook:
    git ai hook install -f
local-install: local-install-hook
    cargo install --debug --path .

test: (docker-run RUST_IMG "cargo test --all")

docker-exec +CMD:
    docker run --rm -v $PWD:/git-ai -w /git-ai git-ai:latest {{CMD}}

docker-build:
    docker build -t git-ai .

docker-run IMG CMD:
    docker run --rm -v $PWD:/git-ai -w /git-ai -it {{IMG}} {{CMD}}
