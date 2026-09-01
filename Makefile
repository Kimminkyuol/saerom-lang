.PHONY: all test clean fmt

all:
	cargo build

test:
	cargo test

fmt:
	cargo fmt

clean:
	cargo clean
