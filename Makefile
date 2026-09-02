.PHONY: all test clean fmt install

PREFIX ?= $(HOME)/.local

all:
	cargo build

test:
	cargo test

fmt:
	cargo fmt

install:
	cargo build --release
	mkdir -p $(PREFIX)/bin $(PREFIX)/lib/saerom
	cp target/release/saeromc $(PREFIX)/bin/
	cp target/release/libsaerom_rt.a $(PREFIX)/lib/saerom/
	@echo "설치됨. $(PREFIX)/bin을 PATH에 등록하세요."

clean:
	cargo clean
