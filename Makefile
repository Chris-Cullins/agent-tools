.PHONY: build fmt clippy test
build:
	cargo build

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

test:
	cargo test
