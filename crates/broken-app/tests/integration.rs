use broken_app::{algo, leak_buffer, normalize, sum_even};

#[test]
fn sums_even_numbers() {
    let nums = [1, 2, 3, 4];
    // Ожидаем корректное суммирование: 2 + 4 = 6.
    assert_eq!(sum_even(&nums), 6);
}

#[test]
fn sums_empty_slice_without_reading_past_end() {
    assert_eq!(sum_even(&[]), 0);
}

#[test]
fn counts_non_zero_bytes() {
    let data = [0_u8, 1, 0, 2, 3];
    assert_eq!(leak_buffer(&data), 3);
}

#[test]
fn dedup_preserves_uniques() {
    let uniq = algo::slow_dedup(&[5, 5, 1, 2, 2, 3]);
    assert_eq!(uniq, vec![1, 2, 3, 5]); // порядок и состав важны
}

#[test]
fn fib_small_numbers() {
    assert_eq!(algo::slow_fib(10), 55);
}

#[test]
fn normalize_simple() {
    assert_eq!(normalize(" Hello World "), "helloworld");
}

#[test]
fn normalize_removes_all_whitespace() {
    assert_eq!(normalize(" Hello\tRust\nWorld "), "hellorustworld");
}

#[test]
fn averages_only_positive() {
    let nums = [-5, 5, 15];
    // Ожидается (5 + 15) / 2 = 10, но текущая реализация делит на все элементы.
    assert!((broken_app::average_positive(&nums) - 10.0).abs() < f64::EPSILON);
}

#[test]
fn average_without_positive_values_is_zero() {
    assert_eq!(broken_app::average_positive(&[-5, -10]), 0.0);
}

#[test]
fn race_increment_counts_every_update() {
    let total = broken_app::concurrency::race_increment(10_000, 8);
    assert_eq!(total, 80_000);
}

#[test]
fn unsafe_helper_returns_original_value() {
    assert_eq!(unsafe { broken_app::use_after_free() }, 42);
}
