GITHUB_USER := "oleander"
GITHUB_REPO := "git-ai"
LOCAL_IMG := "git-ai:latest"
RUST_IMG := "rust:1.76.0"

release: (docker-run RUST_IMG "scripts/release")
test: (docker-run RUST_IMG "tools/test.sh")

local-github-actions:
    act --container-architecture linux/amd64

# Local end-to-end smoke test against ollama (no OpenAI key needed).
# Requires `ollama serve` running. Pulls a tool-calling-capable model, points git-ai
# at ollama's OpenAI-compatible endpoint, and runs a real commit through the hook.
# Note: ollama's support for forced named tool_choice varies by model; if the multi-step
# path can't tool-call it falls back to the single-step/local path. The authoritative
# real-model acceptance test is the OpenAI integration-tests run in CI.
test-ollama MODEL="qwen2.5-coder":
    #!/usr/bin/env bash
    set -euo pipefail
    ollama list | grep -q "{{MODEL}}" || ollama pull "{{MODEL}}"
    cargo install --debug --path .
    git ai config set openai-base-url http://localhost:11434/v1
    git ai config set openai-api-key ollama
    git ai config set model "{{MODEL}}"
    dir="$(mktemp -d)"; trap 'rm -rf "$dir"' EXIT
    git -C "$dir" init -q && git -C "$dir" config user.email t@t.io && git -C "$dir" config user.name t
    printf 'fn main() { println!("hi"); }\n' > "$dir/main.rs"
    printf 'pub fn greet() -> &str { "hi" }\n' > "$dir/lib.rs"
    git -C "$dir" add -A
    git ai hook install --repo "$dir" 2>/dev/null || true
    ( cd "$dir" && git commit --no-edit ) && echo "--- generated message ---" && git -C "$dir" log -1 --pretty=%B

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

# just pr 74 "cargo fmt --all"
# just pr 74 "cargo build"
pr PR_NUMBER CMD:
    docker build --build-arg PR_NUMBER={{PR_NUMBER}} --build-arg GH_TOKEN=$(gh auth token) --target pr-tester -t git-ai-pr-tester .
    docker run -i --rm -e GITHUB_TOKEN=$(gh auth token) git-ai-pr-tester bash -c "{{CMD}}"

sync-pr PR_NUM CMD = "date":
    just pr {{PR_NUM}} "git fetch origin && git merge origin/main --no-edit && {{CMD}} && cargo fmt --check && cargo check && git push origin"
