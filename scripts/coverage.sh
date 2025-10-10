#!/usr/bin/env bash
set -euo pipefail

# Simple coverage runner that prints a text table of unit test coverage.
# Prefers cargo-llvm-cov; falls back to cargo-tarpaulin if available.
#
# Requirements for llvm-cov path:
#   - rustup component add llvm-tools-preview
#   - cargo install cargo-llvm-cov
#
# Examples:
#   scripts/coverage.sh            # summary + per-file table
#   scripts/coverage.sh --html     # also generate HTML report
#   scripts/coverage.sh --open     # open HTML report in browser (if generated)

want_html=false
want_open=false
for arg in "$@"; do
  case "$arg" in
  --html) want_html=true ;;
  --open)
    want_open=true
    want_html=true
    ;;
  *)
    echo "Unknown option: $arg" >&2
    exit 2
    ;;
  esac
done

have() { command -v "$1" >/dev/null 2>&1; }

mkdir -p target/coverage

if have cargo-llvm-cov || cargo llvm-cov --version >/dev/null 2>&1; then
  echo "[coverage] Using cargo-llvm-cov"
  # Run tests with coverage instrumentation and keep artifacts for report
  cargo llvm-cov --workspace --all-features --no-clean --quiet || true

  #echo "Coverage Summary (workspace):"
  #cargo llvm-cov report --summary-only || true

  echo
  #echo "Per-file Coverage Table:"
  #cargo llvm-cov report --text || true

  if $want_html; then
    out_dir="target/coverage/html"
    rm -rf "$out_dir"
    cargo llvm-cov report --all-features \
      --html --output-path "$out_dir" >/dev/null || true
    echo
    echo "HTML report: $out_dir/index.html"
    if $want_open; then
      if [[ "$OSTYPE" == darwin* ]]; then
        open "$out_dir/index.html" || true
      elif command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$out_dir/index.html" || true
      fi
    fi
  fi
  exit 0
fi

if have cargo-tarpaulin; then
  echo "[coverage] Using cargo-tarpaulin"
  # Text output prints a crate summary and hits/coverage
  #cargo tarpaulin --workspace --all-features --out Stdout || true
  exit 0
fi

cat >&2 <<'EOF'
[coverage] No coverage tool found.

Install one of the following:
  1) cargo-llvm-cov (recommended)
     cargo install cargo-llvm-cov
     rustup component add llvm-tools-preview

  2) cargo-tarpaulin
     cargo install cargo-tarpaulin

Then rerun: scripts/coverage.sh
EOF
exit 127
