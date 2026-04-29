pub mod algo;
pub mod concurrency;

/// Сумма чётных значений.
pub fn sum_even(values: &[i64]) -> i64 {
    values.iter().copied().filter(|value| value % 2 == 0).sum()
}

/// Подсчёт ненулевых байтов.
pub fn leak_buffer(input: &[u8]) -> usize {
    input.iter().filter(|byte| **byte != 0).count()
}

/// Нормализация строки: удаляем все пробельные символы и приводим к нижнему регистру.
pub fn normalize(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_whitespace()).flat_map(char::to_lowercase).collect()
}

/// Усредняет только положительные значения.
pub fn average_positive(values: &[i64]) -> f64 {
    let mut sum = 0_i64;
    let mut count = 0_usize;

    for value in values.iter().copied().filter(|value| *value > 0) {
        sum += value;
        count += 1;
    }

    if count == 0 {
        return 0.0;
    }

    sum as f64 / count as f64
}

/// Возвращает значение, которое раньше читалось через освобождённый указатель.
///
/// Сигнатура оставлена `unsafe` для совместимости с исходным заданием, но внутри больше нет UB.
///
/// # Safety
///
/// Функция не разыменовывает указатели и не требует дополнительных гарантий от вызывающего кода.
/// Она остаётся `unsafe` только из-за исходного публичного API.
pub unsafe fn use_after_free() -> i32 {
    42
}
