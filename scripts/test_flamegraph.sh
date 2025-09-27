#!/usr/bin/env bash
set -euo pipefail

# Generate a CPU flamegraph SVG for Rust tests.
#
# Requirements (any one path):
# - cargo-flamegraph (preferred): https://github.com/flamegraph-rs/flamegraph
#   - Linux: uses perf
#   - macOS: uses dtrace (requires sudo)
# - Fallback (Linux only): perf + flamegraph.pl in PATH
#
# Usage examples:
#   scripts/test_flamegraph.sh                          # default: target/flamegraph-tests.svg
#   scripts/test_flamegraph.sh -o out.svg               # custom output path
#   scripts/test_flamegraph.sh --release                # profile release build
#   scripts/test_flamegraph.sh -- test_name_substring   # pass args to test harness

OUT="target/flamegraph-tests.svg"
PROFILE="dev"
TEST_TARGET="integration" # default test harness name (from tests/integration.rs)
PASS_THROUGH=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -o|--output)
      OUT="$2"; shift 2;;
    --release)
      PROFILE="release"; shift;;
    --test)
      TEST_TARGET="$2"; shift 2;;
    --)
      shift; PASS_THROUGH+=("$@"); break;;
    *)
      # Treat remaining as test harness args
      PASS_THROUGH+=("$1"); shift;;
  esac
done

OS=$(uname -s || echo "Unknown")
has() { command -v "$1" >/dev/null 2>&1; }

echo "[flamegraph] output: $OUT"
echo "[flamegraph] profile: $PROFILE"

# Map profile to cargo flags (cargo test vs cargo flamegraph may differ)
PROFILE_FLAG=""
FG_PROFILE_FLAG=""
if [[ "$PROFILE" == "release" ]]; then
  PROFILE_FLAG="--release"
  FG_PROFILE_FLAG="--release"
else
  # Prefer dev for cargo-flamegraph to keep debuginfo without extra config
  FG_PROFILE_FLAG="--dev"
fi

# Ensure build artifacts up to date
cargo test --no-run $PROFILE_FLAG

# Ensure output directory exists
OUT_DIR=$(dirname "$OUT")
mkdir -p "$OUT_DIR"

if has cargo-flamegraph; then
  echo "[flamegraph] Using cargo-flamegraph"
  # Note: --tests profiles the test harness. Sudo may be required on macOS.
  # Ensure flamegraph has symbols in bench/release if used by tool
  export CARGO_PROFILE_BENCH_DEBUG=true
  if [[ "$OS" == "Darwin" ]]; then
    echo "[flamegraph] macOS detected; dtrace typically requires sudo."
    echo "[flamegraph] You may be prompted for your password."
    sudo cargo flamegraph --test "$TEST_TARGET" $FG_PROFILE_FLAG --output "$OUT" -- "${PASS_THROUGH[@]}"
  else
    cargo flamegraph --test "$TEST_TARGET" $FG_PROFILE_FLAG --output "$OUT" -- "${PASS_THROUGH[@]}"
  fi
  echo "[flamegraph] Wrote $OUT"
  exit 0
fi

if [[ "$OS" == "Linux" ]] && has perf && has flamegraph.pl; then
  echo "[flamegraph] Using perf + flamegraph.pl fallback (Linux)"
  # Find first test binary to profile
  if ! has jq; then
    echo "[error] 'jq' is required for fallback path (sudo apt install jq)." >&2
    exit 1
  fi
  # Prefer integration test harness if present
  BIN=$(cargo test --no-run --message-format=json $PROFILE_FLAG \
        | jq -r 'select(.profile.test == true) | .filenames[]' \
        | (grep -E "/integration-[a-f0-9]+$" || true) \
        | head -n1)
  if [[ -z "$BIN" ]]; then
    BIN=$(cargo test --no-run --message-format=json $PROFILE_FLAG \
          | jq -r 'select(.profile.test == true) | .filenames[]' \
          | head -n1)
  fi
  [[ -n "$BIN" ]] || { echo "[error] could not locate test binary" >&2; exit 1; }
  echo "[flamegraph] profiling: $BIN"
  perf record -F 99 -g -- "$BIN" "${PASS_THROUGH[@]}" || true
  perf script | flamegraph.pl > "$OUT"
  echo "[flamegraph] Wrote $OUT"
  exit 0
fi

echo "[error] No supported profiler found. Please install cargo-flamegraph." >&2
echo "        Linux fallback requires: perf, flamegraph.pl, jq" >&2
exit 1
