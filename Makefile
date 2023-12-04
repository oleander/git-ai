install:
	cargo install --path .
build_hook:
	cargo build --bin hook
install_hook: build_hook
	gln -rfs target/debug/hook .git/hooks/prepare-commit-msg
