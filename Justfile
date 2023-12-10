github-actions:
    act --container-architecture linux/amd64

install:
	cargo install --debug --path .
	git ai hook uninstall
	git ai hook install
test:
	cargo test --all
build_hook:
	cargo build --bin hook --release
install_hook: build_hook
	gln -rfs target/release/hook .git/hooks/prepare-commit-msg
simulate:
	./simulate.sh