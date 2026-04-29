# Debugging And Optimization Report

## Root Causes

- `sum_even` used `get_unchecked` with `0..=len`, so every call read one element past the slice.
- `leak_buffer` converted a `Box<[u8]>` into a raw pointer and never reconstructed the box.
- `normalize` removed only `' '` and ignored tabs/newlines/other whitespace.
- `average_positive` divided the sum of all values by the full input length.
- `use_after_free` read from a raw pointer after freeing the `Box`.
- `race_increment` used `static mut` across threads without synchronization.
- `slow_fib` used exponential recursion.
- `slow_dedup` repeatedly scanned the output and sorted after every insertion.

## Regression Tests

Added tests for:

- empty `sum_even` input;
- all-whitespace normalization;
- no-positive-value average;
- thread-safe increment count;
- `use_after_free` compatibility behavior.

The original integration tests remain in place.

## Verification

Completed:

- `cargo test -p broken-app`: 11 integration tests passed.
- `cargo +nightly miri test -p broken-app`: 11 integration tests passed under Miri.
- ASan on a temporary slim copy without bench/dev dependencies: 11 integration tests passed.
- Valgrind in Docker: 11 integration tests passed, `definitely lost: 0`, `indirectly lost: 0`,
  `ERROR SUMMARY: 0`.
- `cargo bench -p broken-app --bench baseline`: completed on the fixed crate.

Environment limits:

- TSan with `-Zbuild-std` on nightly macOS/aarch64 produced a SIGSEGV before running tests.

## Tool Notes

- Use ordinary tests first to identify behavior regressions and to create focused regression cases.
- Use Miri for UB in unsafe code: out-of-bounds, dangling pointers, invalid aliasing, and
  use-after-free.
- Use Valgrind for Linux memory checks. Treat `definitely lost`, `indirectly lost`, invalid reads,
  invalid writes, and non-zero `ERROR SUMMARY` as blockers.
- Use ASan as a faster memory-safety signal when the local toolchain supports loading the sanitizer
  runtime correctly.
- Use TSan for data races when the target/toolchain supports it; inspect both reported stacks for
  the conflicting accesses.
- Use benchmarks only after correctness is green. Compare identical inputs before and after each
  optimization.

## Benchmark Summary

The before/after comparison uses the same temporary benchmark runner over the assignment's hot
algorithm paths.

| Case | Before | After | Improvement |
| --- | ---: | ---: | ---: |
| `fib_32_x1` | 4.477833 ms | 83 ns | about 53950x |
| `dedup_4000_x1` | 1.407916 ms | 54.583 us | about 25.8x |

The original `sum_even` benchmark from `broken-app` is not a reliable baseline because the function
has undefined behavior in release mode.
