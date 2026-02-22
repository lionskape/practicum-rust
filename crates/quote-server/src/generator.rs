//! Генератор котировок — создаёт синтетические котировки на основе модели случайного блуждания.

use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use quote_common::StockQuote;
use rand::Rng;

/// Генератор синтетических котировок для набора тикеров.
///
/// Хранит внутреннее состояние (текущие цены) и на каждом тике создаёт
/// новые котировки с помощью случайного блуждания.
///
/// # Примеры
///
/// ```
/// use quote_server::generator::QuoteGenerator;
///
/// fn main() {
///     let tickers = vec!["AAPL".into(), "TSLA".into()];
///     let mut generator = QuoteGenerator::new(&tickers);
///
///     let quotes = generator.generate_all();
///     assert_eq!(quotes.len(), 2);
///     assert!(quotes.iter().all(|q| q.price > 0.0));
/// }
/// ```
pub struct QuoteGenerator {
    /// Текущая цена для каждого тикера.
    prices: HashMap<String, f64>,
    /// Генератор случайных чисел.
    rng: rand::rngs::ThreadRng,
}

impl QuoteGenerator {
    /// Создаёт новый генератор со случайными начальными ценами для каждого тикера.
    ///
    /// Начальные цены выбираются случайно в диапазоне $10–$500.
    pub fn new(tickers: &[String]) -> Self {
        let mut rng = rand::rng();
        let prices = tickers
            .iter()
            .map(|t| {
                let initial_price = rng.random_range(10.0..500.0);
                (t.clone(), initial_price)
            })
            .collect();
        Self { prices, rng }
    }

    /// Генерирует свежую порцию котировок для ВСЕХ отслеживаемых тикеров.
    ///
    /// Каждый вызов продвигает симуляцию на один шаг:
    /// - Применяет случайное блуждание к каждой цене (малое процентное изменение).
    /// - Генерирует случайный объём торгов.
    /// - Проставляет текущую временну́ю метку.
    ///
    /// Возвращает `Vec<StockQuote>` с одной записью на тикер.
    pub fn generate_all(&mut self) -> Vec<StockQuote> {
        self.prices
            .iter_mut()
            .for_each(|(_, p)| *p = (*p * self.rng.random_range(0.98..1.02)).max(0.01));

        let ts_millis: u64 =
            SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis()
                as u64;

        self.prices
            .iter()
            .map(|(ticker, price)| StockQuote {
                ticker: ticker.clone(),
                price: *price,
                volume: self.rng.random_range(100..10_000),
                timestamp: ts_millis,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tickers() -> Vec<String> {
        vec!["AAPL".into(), "TSLA".into(), "MSFT".into()]
    }

    #[test]
    fn generates_quotes_for_all_tickers() {
        let mut generator = QuoteGenerator::new(&sample_tickers());
        let quotes = generator.generate_all();
        assert_eq!(quotes.len(), 3, "should produce one quote per ticker");
    }

    #[test]
    fn prices_are_positive() {
        let mut generator = QuoteGenerator::new(&sample_tickers());
        for _ in 0..100 {
            for q in generator.generate_all() {
                assert!(q.price > 0.0, "price must stay positive: {}", q.ticker);
            }
        }
    }

    #[test]
    fn prices_change_between_ticks() {
        let mut generator = QuoteGenerator::new(&sample_tickers());
        let first = generator.generate_all();
        let second = generator.generate_all();
        // Хотя бы у одного тикера цена должна измениться (статистически гарантировано)
        let any_changed =
            first.iter().zip(second.iter()).any(|(a, b)| (a.price - b.price).abs() > f64::EPSILON);
        assert!(any_changed, "prices should change between ticks");
    }

    #[test]
    fn volume_is_positive() {
        let mut generator = QuoteGenerator::new(&sample_tickers());
        for q in generator.generate_all() {
            assert!(q.volume > 0, "volume must be positive: {}", q.ticker);
        }
    }

    #[test]
    fn timestamp_is_recent() {
        let mut generator = QuoteGenerator::new(&sample_tickers());
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        for q in generator.generate_all() {
            assert!(q.timestamp <= now_ms + 1000, "timestamp too far in the future");
            assert!(q.timestamp >= now_ms - 5000, "timestamp too far in the past");
        }
    }
}
