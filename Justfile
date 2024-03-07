set shell := ["bash", "-cu"]

GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"
LOCAL_IMG := "git-ai:latest"
RUST_IMG := "rust:1.76.0"

release: (docker-run RUST_IMG "scripts/release")
test: (docker-run RUST_IMG "tools/test.sh")

local-github-actions:
    act --container-architecture linux/amd64

local-install-hook:
    git ai hook install -f

local-install: local-install-hook
    cargo install --debug --path .

docker-build:
    docker build -t git-ai .

docker-run IMG CMD:
    docker run --rm -v $PWD:/git-ai -w /git-ai -it {{IMG}} {{CMD}}
