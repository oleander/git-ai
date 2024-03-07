set shell := ["bash", "-cu"]

GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"

# Use docker run with --rm for tasks that require Rust environment
# Mount the current directory into /git-ai in the container
# Use the image git-ai:latest for all tasks
docker-cmd := "docker run --rm -v $PWD:/git-ai -w /git-ai git-ai:latest"

github-actions:
    $(docker-cmd) act --container-architecture linux/amd64

install:
    $(docker-cmd) cargo install --debug  --path .

test: docker-build
    $(docker-cmd) cargo test --all

build-hook:
    $(docker-cmd) cargo build --bin hook

install-hook: install
    $(docker-cmd) git ai hook install -f

release:
    $(docker-cmd) bash -c "\
    cargo update && \
    git add Cargo.lock Cargo.toml && \
    git commit --no-edit && \
    version=$$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].version' | tr -d '\n') && \
    echo 'Releasing $$version' && \
    git tag -a v$$version -m 'Release v$$version' && \
    git push origin v$$version && \
    git push origin main && \
    git push --tags"

clean:
    $(docker-cmd) cargo clean

docker-build:
    docker build -t git-ai .
docker-run +CMD: docker-build
    docker run --rm -v $PWD:/git-ai -w /git-ai -it git-ai:latest {{CMD}}
