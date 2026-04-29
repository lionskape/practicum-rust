# Module 5: Debugging And Optimization

This crate contains the fixed `broken-app` from the module 5 assignment.

## Fixed Issues

- Replaced off-by-one `get_unchecked` access in `sum_even` with a safe iterator pass.
- Removed the allocation leak in `leak_buffer`.
- Reworked `normalize` to remove all whitespace, not only literal spaces.
- Fixed `average_positive` to average only positive values and return `0.0` when none exist.
- Removed use-after-free from `use_after_free` while preserving the original public signature.
- Replaced `static mut` counter updates with `AtomicU64`.
- Optimized `slow_fib` from recursive exponential time to iterative linear time.
- Optimized `slow_dedup` by using a `HashSet` and sorting once.

## Commands

```bash
cargo test -p broken-app
cargo +nightly miri test -p broken-app
cargo bench -p broken-app --bench baseline
```

The root workspace CI also covers this crate:

```bash
cargo ci
```

## Debugging Tools

Use `cargo test -p broken-app` for behavior regressions, `cargo +nightly miri test -p broken-app`
for UB, and Valgrind in Docker for Linux memory checks:

```bash
docker run --rm -v "$PWD":/workspace -w /workspace rust:bookworm bash -lc '
  set -euo pipefail
  export PATH=/usr/local/cargo/bin:$PATH
  apt-get update
  DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends valgrind
  CARGO_TARGET_DIR=/tmp/practicum-target cargo test -p broken-app --tests --no-run
  test_bin=$(find /tmp/practicum-target/debug/deps -maxdepth 1 -type f -name "integration-*" -perm /111 | head -n 1)
  valgrind --leak-check=full \
    --show-leak-kinds=definite,indirect \
    --errors-for-leak-kinds=definite,indirect \
    --error-exitcode=1 \
    "$test_bin" --test-threads=1
'
```

Good Valgrind output has `definitely lost: 0`, `indirectly lost: 0`, and `ERROR SUMMARY: 0`.
See the module 5 docs page for the full tool guide and interpretation notes.

## Artifacts

The `artifacts/` directory contains test, Miri, sanitizer, Valgrind availability, and benchmark
logs. `REPORT.md` summarizes the root causes and before/after timings.
