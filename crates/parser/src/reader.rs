//! Потоковый reader для транзакций.
//!
//! Предоставляет [`TransactionReader`] — итератор, который читает
//! транзакции из любого источника, реализующего [`Read`].

use std::{
    io::{BufReader, Read},
    marker::PhantomData,
};

use crate::{
    serde::{Result, SerdeFormat},
    transaction::Transaction,
};

/// Потоковый reader для транзакций.
///
/// Реализует [`Iterator`] для чтения транзакций из потока.
/// Использует [`BufReader`] для буферизации, что необходимо
/// для корректной работы текстового формата.
///
/// # Type Parameters
///
/// - `R`: источник данных (реализует [`Read`])
/// - `F`: формат (реализует [`SerdeFormat`])
///
/// # Пример
///
/// ```ignore
/// use parser::reader::TransactionReader;
/// use parser::serde::Binary;
/// use std::fs::File;
///
/// let file = File::open("transactions.bin")?;
/// let reader = TransactionReader::<_, Binary>::new(file);
///
/// for result in reader {
///     let tx = result?;
///     println!("{:?}", tx);
/// }
/// ```
pub struct TransactionReader<R, F> {
    inner: BufReader<R>,
    _format: PhantomData<F>,
    /// Счётчик прочитанных записей.
    records_read: usize,
    /// Флаг достижения EOF или ошибки.
    finished: bool,
    /// Флаг: был ли пропущен заголовок (для CSV).
    header_skipped: bool,
}

impl<R: Read, F: SerdeFormat> TransactionReader<R, F> {
    /// Создаёт новый reader.
    ///
    /// Входной reader оборачивается в [`BufReader`] для буферизации.
    pub fn new(reader: R) -> Self {
        Self {
            inner: BufReader::new(reader),
            _format: PhantomData,
            records_read: 0,
            finished: false,
            header_skipped: false,
        }
    }

    /// Возвращает количество успешно прочитанных записей.
    #[must_use]
    pub fn records_read(&self) -> usize {
        self.records_read
    }

    /// Получает ссылку на внутренний reader.
    #[must_use]
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }

    /// Извлекает внутренний reader.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}

impl<R: Read, F: SerdeFormat> Iterator for TransactionReader<R, F> {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Skip header on first read (for CSV format)
        if !self.header_skipped {
            self.header_skipped = true;
            if let Err(e) = F::skip_header(&mut self.inner) {
                self.finished = true;
                return Some(Err(e));
            }
        }

        match F::read_one(&mut self.inner) {
            Ok(Some(tx)) => {
                self.records_read += 1;
                Some(Ok(tx))
            }
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(e) => {
                self.finished = true; // Остановка при ошибке
                Some(Err(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::{
        serde::{Binary, Text, binary},
        transaction::{TransactionStatus, TransactionType},
    };

    #[test]
    fn test_read_multiple_text_transactions() {
        let input = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 42
AMOUNT: 100
TIMESTAMP: 1000
STATUS: SUCCESS
DESCRIPTION: "First"

TX_ID: 2
TX_TYPE: TRANSFER
FROM_USER_ID: 42
TO_USER_ID: 100
AMOUNT: 50
TIMESTAMP: 2000
STATUS: PENDING
DESCRIPTION: "Second"
"#;
        let reader = TransactionReader::<_, Text>::new(Cursor::new(input));
        let txs: Result<Vec<_>> = reader.collect();
        let txs = txs.unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].tx_id, 1);
        assert_eq!(txs[0].tx_type, TransactionType::Deposit);
        assert_eq!(txs[1].tx_id, 2);
        assert_eq!(txs[1].tx_type, TransactionType::Transfer);
    }

    #[test]
    fn test_read_binary_transactions_with_reader() {
        let tx1 = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 42,
            amount: 100,
            timestamp: 1000,
            status: TransactionStatus::Success,
            description: "First".to_string(),
        };
        let tx2 = Transaction {
            tx_id: 2,
            tx_type: TransactionType::Transfer,
            from_user_id: 42,
            to_user_id: 100,
            amount: 50,
            timestamp: 2000,
            status: TransactionStatus::Pending,
            description: "Second".to_string(),
        };

        let mut buffer = Vec::new();
        binary::write_one(&mut buffer, &tx1).unwrap();
        binary::write_one(&mut buffer, &tx2).unwrap();

        let reader = TransactionReader::<_, Binary>::new(Cursor::new(buffer));
        let txs: Result<Vec<_>> = reader.collect();
        let txs = txs.unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].tx_id, 1);
        assert_eq!(txs[1].tx_id, 2);
    }

    #[test]
    fn test_records_read_counter() {
        let tx = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 1,
            amount: 100,
            timestamp: 1000,
            status: TransactionStatus::Success,
            description: "Test".to_string(),
        };

        let mut buffer = Vec::new();
        binary::write_one(&mut buffer, &tx).unwrap();

        let mut reader = TransactionReader::<_, Binary>::new(Cursor::new(buffer));

        assert_eq!(reader.records_read(), 0);
        let _ = reader.next();
        assert_eq!(reader.records_read(), 1);
        let _ = reader.next(); // EOF
        assert_eq!(reader.records_read(), 1);
    }
}
