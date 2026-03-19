SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

CRATE := $(shell sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -n1)
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
TAG := v$(VERSION)

.PHONY: help build test format release clean

help:
	@echo "Common targets:"
	@echo "  make build        fmt-check + clippy + cargo test --all-features --workspace"
	@echo "  make test         alias of build"
	@echo "  make format       cargo fmt --all"
	@echo "  make release      release checks + test + cargo publish + GitHub release"
	@echo "  make clean        cargo clean"

build:
	cargo +nightly fmt --all -- --check
	cargo +nightly clippy --all-targets --all-features -- -D warnings
	cargo +nightly test --all-features --workspace

test:
	$(MAKE) build

format:
	cargo +nightly fmt --all

release:
	@command -v gh >/dev/null 2>&1 || { echo "gh CLI is required"; exit 1; }
	@gh auth status >/dev/null 2>&1 || { echo "gh auth is required (run: gh auth login)"; exit 1; }
	@test -z "$$(git status --porcelain)" || { echo "git worktree is dirty; commit/stash first"; exit 1; }
	@git rev-parse -q --verify "refs/tags/$(TAG)" >/dev/null && { echo "tag $(TAG) already exists"; exit 1; } || true
	$(MAKE) test
	@echo "Releasing $(CRATE) $(VERSION)"
	cargo publish
	git tag -a "$(TAG)" -m "$(CRATE) $(VERSION)"
	git push origin "$(TAG)"
	gh release create "$(TAG)" --title "$(CRATE) $(VERSION)" --generate-notes
	@echo "Release complete: $(TAG)"

clean:
	cargo +nightly clean
