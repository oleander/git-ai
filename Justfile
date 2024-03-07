set shell := ["bash", "-cu"]

GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"
DOCKER_TAG := "git-ai"

github-actions: (docker-cmd "act --container-architecture linux/amd64")

install: (docker-cmd "cargo install --debug --path .")

test: docker-build (docker-cmd "cargo test --all")

build-hook: (docker-cmd "cargo build --bin hook")

install-hook: install (docker-cmd "git ai hook install -f")

clean: (docker-cmd "cargo clean")

docker-cmd +CMD: docker-build
    # docker run --rm -v $PWD:/git-ai -w /git-ai -it {{DOCKER_TAG}}:latest {{CMD}}
    docker run --rm -v $PWD:/git-ai -w /git-ai git-ai:latest {{CMD}}

docker-build:
    docker build -t {{DOCKER_TAG}} .
docker-clear-cache:
    docker builder prune --all --force
# release: (docker-cmd "cargo update && \
#             git add Cargo.lock Cargo.toml && \
#             git commit --no-edit && \
#             version=$$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].version' | tr -d '\n') && \
#             echo 'Releasing $$version' && \
#             git tag -a v$$version -m 'Release v$$version' && \
#             git push origin v$$version && \
#             git push origin main && \
#             git push --tags"
