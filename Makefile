BIN     := speed-reader
PREFIX  ?= $(HOME)/.local
DIST    := dist
VERSION := $(shell awk -F\" '/^version/ {print $$2; exit}' Cargo.toml)

# ---- native (same OS you're on) ----

.PHONY: all build run install uninstall clean help

all: build

build:
	cargo build --release
	@echo "built: target/release/$(BIN)"

run: build
	./target/release/$(BIN) sample.md

install: build
	install -d $(PREFIX)/bin
	install -m 0755 target/release/$(BIN) $(PREFIX)/bin/$(BIN)
	@echo "installed: $(PREFIX)/bin/$(BIN)"

uninstall:
	rm -f $(PREFIX)/bin/$(BIN)

clean:
	cargo clean
	rm -rf $(DIST)

# ---- cross builds ----
# One-time setup: `make setup-cross` (Linux/macOS)

.PHONY: setup-cross mac mac-arm mac-intel linux linux-x86 linux-arm dist

setup-cross:
	@echo "installing cargo-zigbuild…"
	cargo install --locked cargo-zigbuild
	@if command -v brew >/dev/null 2>&1; then \
	  echo "installing zig via brew…"; brew install zig; \
	else \
	  echo "installing zig via pip (ziglang package)…"; \
	  pip3 install --user --break-system-packages ziglang; \
	  mkdir -p $(HOME)/.local/bin; \
	  printf '#!/bin/sh\nexec python3 -m ziglang "$$@"\n' > $(HOME)/.local/bin/zig; \
	  chmod +x $(HOME)/.local/bin/zig; \
	  echo "→ ensure $(HOME)/.local/bin is on your PATH"; \
	fi
	@zig version >/dev/null 2>&1 && echo "setup complete" \
	  || echo "WARNING: 'zig' not on PATH — add \$$HOME/.local/bin to PATH"

mac: mac-arm mac-intel
linux: linux-x86 linux-arm

mac-arm:
	rustup target add aarch64-apple-darwin
	cargo zigbuild --release --target aarch64-apple-darwin

mac-intel:
	rustup target add x86_64-apple-darwin
	cargo zigbuild --release --target x86_64-apple-darwin

linux-x86:
	rustup target add x86_64-unknown-linux-gnu
	cargo zigbuild --release --target x86_64-unknown-linux-gnu

linux-arm:
	rustup target add aarch64-unknown-linux-gnu
	cargo zigbuild --release --target aarch64-unknown-linux-gnu

TARGETS := aarch64-apple-darwin \
           x86_64-apple-darwin \
           x86_64-unknown-linux-gnu \
           aarch64-unknown-linux-gnu

dist: mac linux
	@mkdir -p $(DIST)
	@for t in $(TARGETS); do \
	  out=$(BIN)-$(VERSION)-$$t ; \
	  cp target/$$t/release/$(BIN) $(DIST)/$$out ; \
	  tar -C $(DIST) -czf $(DIST)/$$out.tar.gz $$out ; \
	  rm $(DIST)/$$out ; \
	  echo "  $(DIST)/$$out.tar.gz" ; \
	done

help:
	@echo "native:"
	@echo "  make              release build for host"
	@echo "  make run          build + open sample.md"
	@echo "  make install      install to $(PREFIX)/bin"
	@echo "  make uninstall    remove installed binary"
	@echo "  make clean        cargo clean + rm $(DIST)/"
	@echo
	@echo "cross (run 'make setup-cross' once):"
	@echo "  make setup-cross  install zig + cargo-zigbuild"
	@echo "  make mac          arm64 + x86_64 macOS"
	@echo "  make mac-arm      aarch64-apple-darwin (Apple Silicon)"
	@echo "  make mac-intel    x86_64-apple-darwin"
	@echo "  make linux        x86_64 + aarch64 Linux"
	@echo "  make linux-x86    x86_64-unknown-linux-gnu"
	@echo "  make linux-arm    aarch64-unknown-linux-gnu"
	@echo "  make dist         build all four + tarballs in $(DIST)/"
