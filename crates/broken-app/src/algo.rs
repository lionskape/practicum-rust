use std::collections::HashSet;

/// Возвращает отсортированный список уникальных значений.
pub fn slow_dedup(values: &[u64]) -> Vec<u64> {
    let mut seen = HashSet::with_capacity(values.len());
    let mut out = Vec::with_capacity(values.len());

    for &value in values {
        if seen.insert(value) {
            out.push(value);
        }
    }

    out.sort_unstable();
    out
}

/// Итеративная реализация числа Фибоначчи.
pub fn slow_fib(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut previous = 0;
            let mut current = 1;

            for _ in 2..=n {
                let next = previous + current;
                previous = current;
                current = next;
            }

            current
        }
    }
}
