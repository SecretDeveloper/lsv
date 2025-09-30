#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Option A: use core.hooksPath pointing to .githooks (recommended)
git -C "$ROOT_DIR" config core.hooksPath .githooks

# Ensure the hook is executable
chmod +x "$ROOT_DIR/.githooks/pre-commit"
if [ -f "$ROOT_DIR/.githooks/pre-push" ]; then
  chmod +x "$ROOT_DIR/.githooks/pre-push"
fi

echo "Git hooks installed."
echo "- pre-commit: cargo fmt --all -- --check"
echo "- pre-push:   cargo clippy --all-targets --all-features -- -D warnings"
echo "To uninstall: git config --unset core.hooksPath"
