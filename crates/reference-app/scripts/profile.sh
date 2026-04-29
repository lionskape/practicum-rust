#!/usr/bin/env bash
set -euo pipefail

cargo build --release
perf record -g ./target/release/demo || true
perf report
