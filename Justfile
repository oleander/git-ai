GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"
LOCAL_IMG := "git-ai:latest"
RUST_IMG := "rust:1.76.0"

release: (docker-run RUST_IMG "scripts/release")
test: (docker-run RUST_IMG "tools/test.sh")

local-github-actions:
    act --container-architecture linux/amd64

local-install:
    cargo install --debug --path .
    git ai hook uninstall || true
    git ai hook install

docker-build:
    docker build -t git-ai .

docker-run IMG CMD:
    docker run --rm -v $PWD:/git-ai -w /git-ai -it {{IMG}} {{CMD}}

cd_local:
    act

integration-test:
    source .secrets
    docker build -t git-ai-test .
    docker run --rm git-ai-test -e OPENAI_API_KEY=$OPENAI_API_KEY

pr CMD:
    docker build --target pr-tester -t git-ai-pr-tester .
    docker run -i --rm -e GITHUB_TOKEN=$(gh auth token) git-ai-pr-tester bash -c "{{CMD}}"
