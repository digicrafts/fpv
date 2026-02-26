#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-tests/fixtures/navigation}"
printf "Benchmarking single-layer navigation start-up on %s\n" "$ROOT"
/usr/bin/time -f "elapsed=%E rss=%MKB" cargo run --quiet -- "$ROOT" </dev/null >/dev/null 2>&1 || true
printf "Navigation benchmark completed\n"
