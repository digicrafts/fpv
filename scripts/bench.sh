#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-tests/fixtures}"

printf "Running fpv tree benchmark on %s\n" "$ROOT"
/usr/bin/time -f "elapsed=%E rss=%MKB" cargo run --quiet -- "$ROOT" </dev/null >/dev/null 2>&1 || true
printf "Benchmark finished\n"
