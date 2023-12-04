install:
	cargo build --release --bin git-ai-hook
	cargo build --release --bin git-ai
	cargo install --path . --bin git-ai
	git-ai install
build_hook:
	cargo build --bin hook --release
install_hook: build_hook
	gln -rfs target/release/hook .git/hooks/prepare-commit-msg