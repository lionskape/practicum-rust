#!/usr/bin/env bash
set -euo pipefail

# Бенчмарки референса (для ориентира).
cargo bench --bench baseline > artifacts/reference_baseline.txt
