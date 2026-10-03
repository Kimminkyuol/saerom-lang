.PHONY: all test clean fmt install

PREFIX ?= $(HOME)/.local

all:
	cargo build

# 테스트가 런타임 .a 를 링크하므로 먼저 빌드
test:
	cargo build
	cargo test

fmt:
	cargo fmt

install:
	cargo build --release
	mkdir -p $(PREFIX)/bin $(PREFIX)/lib/saerom
	cp target/release/saeromc $(PREFIX)/bin/
	cp target/release/libsaerom_rt.a $(PREFIX)/lib/saerom/
	mkdir -p $(PREFIX)/lib/saerom/std
	cp std/*.sr $(PREFIX)/lib/saerom/std/
	@echo "설치됨. $(PREFIX)/bin을 PATH에 등록하세요."

clean:
	cargo clean
