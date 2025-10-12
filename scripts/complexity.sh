#!/usr/bin/env bash
set -euo pipefail

# Code complexity summary using Clippy (cognitive complexity)
#
# This script runs `cargo clippy` and parses JSON messages to build a table of
# functions reported by the `clippy::cognitive_complexity` lint. By default,
# Clippy only reports functions exceeding the configured threshold (default 25).
# To see more, set a lower threshold in clippy.toml, e.g.:
#   cognitive-complexity-threshold = 10
#
# Requirements: cargo, jq

root_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root_dir"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found in PATH" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq not found in PATH (required to parse clippy JSON)" >&2
  exit 1
fi

tmp_json="$(mktemp)"
trap 'rm -f "$tmp_json"' EXIT

# Run clippy and capture JSON diagnostics. Use warnings (not deny) so exit code
# stays zero while still emitting the lint messages we need.
echo "Running: cargo clippy (this may take a moment)..." >&2
cargo clippy \
  --all-targets --all-features \
  --message-format=json \
  -- -A warnings -W clippy::cognitive_complexity >"$tmp_json"

# Extract cognitive complexity diagnostics
rows=$(jq -r '
  select(.reason=="compiler-message")
  | .message as $m
  | select($m.code.code=="clippy::cognitive_complexity")
  | {
      file: ($m.spans[0].file_name // ""),
      line: ($m.spans[0].line_start // 0),
      msg:  ($m.message // ""),
      complexity: (($m.message | capture("complexity of (?<n>[0-9]+)").n) // null),
      threshold:  (($m.message | capture("threshold:? (?<t>[0-9]+)").t) // null)
    }
  | [ .file, (.line|tostring), (.complexity//""), (.threshold//""), .msg ]
  | @tsv
' "$tmp_json")

if [[ -z "$rows" ]]; then
  echo "No clippy cognitive complexity diagnostics found." >&2
  echo "Tip: lower cognitive-complexity-threshold in clippy.toml to surface more functions." >&2
  exit 0
fi

# Header
printf "%-60s %6s %8s %10s %s\n" "File" "Line" "Complex" "Threshold" "Message"
printf "%s\n" "$(printf '%.0s-' {1..120})"

# Sort by complexity desc, then file
echo "$rows" | sort -t$'\t' -k3,3nr -k1,1 | while IFS=$'\t' read -r file line cc thr msg; do
  rel="${file#"$root_dir/"}"
  printf "%-60s %6s %8s %10s %s\n" "$rel" "$line" "${cc:-}" "${thr:-}" "$msg"
done

exit 0
