//! Потоковый writer для транзакций.
//!
//! Предоставляет [`TransactionWriter`] для записи транзакций
//! в любой тип, реализующий [`Write`].

use std::{
    io::{BufWriter, Write},
    marker::PhantomData,
};

use crate::{
    serde::{Result, SerdeFormat},
    transaction::Transaction,
};

/// Потоковый writer для транзакций.
///
/// Использует буферизацию для эффективного I/O.
///
/// # Type Parameters
///
/// - `W`: целевой поток (реализует [`Write`])
/// - `F`: формат (реализует [`SerdeFormat`])
///
/// # Пример
///
/// ```ignore
/// use parser::writer::TransactionWriter;
/// use parser::serde::Text;
/// use std::fs::File;
///
/// let file = File::create("output.txt")?;
/// let mut writer = TransactionWriter::<_, Text>::new(file);
///
/// writer.write_header()?;
/// writer.write(&tx1)?;
/// writer.write(&tx2)?;
/// writer.flush()?;
/// ```
pub struct TransactionWriter<W: Write, F: SerdeFormat> {
    inner: BufWriter<W>,
    _format: PhantomData<F>,
    /// Счётчик записанных транзакций.
    records_written: usize,
    /// Флаг: записан ли заголовок.
    header_written: bool,
}

impl<W: Write, F: SerdeFormat> TransactionWriter<W, F> {
    /// Создаёт новый writer.
    pub fn new(writer: W) -> Self {
        Self {
            inner: BufWriter::new(writer),
            _format: PhantomData,
            records_written: 0,
            header_written: false,
        }
    }

    /// Создаёт writer с указанным размером буфера.
    pub fn with_capacity(capacity: usize, writer: W) -> Self {
        Self {
            inner: BufWriter::with_capacity(capacity, writer),
            _format: PhantomData,
            records_written: 0,
            header_written: false,
        }
    }

    /// Записывает заголовок формата (если он есть).
    ///
    /// Для CSV записывает строку с именами колонок.
    /// Для других форматов — no-op.
    /// Может вызываться несколько раз, но заголовок записывается только один раз.
    pub fn write_header(&mut self) -> Result<()> {
        if !self.header_written {
            F::write_header(&mut self.inner)?;
            self.header_written = true;
        }
        Ok(())
    }

    /// Записывает одну транзакцию.
    pub fn write(&mut self, tx: &Transaction) -> Result<()> {
        F::write_one(&mut self.inner, tx)?;
        self.records_written += 1;
        Ok(())
    }

    /// Записывает несколько транзакций.
    pub fn write_all(&mut self, txs: &[Transaction]) -> Result<()> {
        for tx in txs {
            self.write(tx)?;
        }
        Ok(())
    }

    /// Принудительно сбрасывает буфер.
    pub fn flush(&mut self) -> Result<()> {
        self.inner.flush()?;
        Ok(())
    }

    /// Возвращает количество записанных транзакций.
    #[must_use]
    pub fn records_written(&self) -> usize {
        self.records_written
    }

    /// Получает ссылку на внутренний writer.
    #[must_use]
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    /// Извлекает внутренний writer (с предварительным flush).
    ///
    /// Возвращает ошибку, если flush не удался.
    pub fn into_inner(self) -> std::result::Result<W, std::io::IntoInnerError<BufWriter<W>>> {
        self.inner.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        serde::Text,
        transaction::{TransactionStatus, TransactionType},
    };

    fn sample_transaction() -> Transaction {
        Transaction {
            tx_id: 42,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 100,
            amount: 5000,
            timestamp: 1700000000000,
            status: TransactionStatus::Success,
            description: "Test".to_string(),
        }
    }

    #[test]
    fn test_write_text_format() {
        let mut output = Vec::new();
        {
            let mut writer = TransactionWriter::<_, Text>::new(&mut output);
            writer.write(&sample_transaction()).unwrap();
            writer.flush().unwrap();
        }

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("TX_ID: 42"));
        assert!(result.contains("TX_TYPE: DEPOSIT"));
    }

    #[test]
    fn test_records_written_counter() {
        let mut output = Vec::new();
        let mut writer = TransactionWriter::<_, Text>::new(&mut output);

        assert_eq!(writer.records_written(), 0);
        writer.write(&sample_transaction()).unwrap();
        assert_eq!(writer.records_written(), 1);
        writer.write(&sample_transaction()).unwrap();
        assert_eq!(writer.records_written(), 2);
    }
}
