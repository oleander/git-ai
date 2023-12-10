install:
	cargo build --release --bin git-ai-hook --no-default-features
	cargo build --release --bin git-ai --no-default-features
	cargo install --path . --bin git-ai --no-default-features
	git-ai install
test:
	cargo test --all
build_hook:
	cargo build --bin hook --release
install_hook: build_hook
	gln -rfs target/release/hook .git/hooks/prepare-commit-msg
simulate:
	./simulate.sh