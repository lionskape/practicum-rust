//! Библиотека парсинга форматов транзакций YPBank.
//!
//! Этот крейт предоставляет структуры данных и парсеры для работы
//! с файлами транзакций YPBank в трёх форматах:
//!
//! - **YPBankText** — человекочитаемый текстовый формат с парами ключ-значение
//! - **YPBankBin** — компактный бинарный формат с магическим заголовком
//! - **YPBankCsv** — стандартный формат CSV
//!
//! # Быстрый старт
//!
//! ```
//! use parser::transaction::{Transaction, TransactionStatus, TransactionType};
//!
//! let tx = Transaction {
//!     tx_id: 1234567890123456,
//!     tx_type: TransactionType::Deposit,
//!     from_user_id: 0,
//!     to_user_id: 9876543210987654,
//!     amount: 10000,
//!     timestamp: 1633036800000,
//!     status: TransactionStatus::Success,
//!     description: "Пополнение через терминал".to_string(),
//! };
//!
//! assert_eq!(tx.tx_type, TransactionType::Deposit);
//! assert_eq!(tx.from_user_id, 0);
//! ```

pub mod transaction;
