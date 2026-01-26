//! Библиотека парсинга форматов транзакций YPBank.
//!
//! Этот крейт предоставляет структуры данных и парсеры для работы
//! с файлами транзакций YPBank в двух форматах:
//!
//! - **YPBankText** — человекочитаемый текстовый формат с парами ключ-значение
//! - **YPBankBin** — компактный бинарный формат с магическим заголовком
//!
//! # Быстрый старт
//!
//! ```
//! use parser::prelude::*;
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
//!
//! # Чтение транзакций (streaming)
//!
//! ```ignore
//! use parser::prelude::*;
//! use std::fs::File;
//!
//! let file = File::open("transactions.bin")?;
//! let reader = TransactionReader::<_, Binary>::new(file);
//!
//! for result in reader {
//!     let tx = result?;
//!     println!("{:?}", tx);
//! }
//! ```
//!
//! # Альтернативно: прямое использование serde модуля
//!
//! ```ignore
//! use parser::serde::{binary, text};
//!
//! // Binary format
//! for tx in binary::iter_reader(file) {
//!     let tx: Transaction = tx?;
//!     println!("{:?}", tx);
//! }
//!
//! // Text format
//! let tx: Transaction = text::from_str(&text_data)?;
//! ```
//!
//! # Конверсия форматов
//!
//! ```ignore
//! use parser::convert::convert;
//! use parser::serde::{Text, Binary};
//! use std::fs::File;
//!
//! let input = File::open("input.txt")?;
//! let output = File::create("output.bin")?;
//! convert::<_, _, Text, Binary>(input, output)?;
//! ```

pub mod error;
pub mod reader;
pub mod serde;
pub mod transaction;
pub mod writer;

/// Prelude для удобного импорта часто используемых типов.
///
/// ```
/// use parser::prelude::*;
/// ```
pub mod prelude {
    // Re-export serde submodules for convenience
    pub use crate::{
        reader::TransactionReader,
        serde::{Binary, Csv, Format, Result as SerdeResult, SerdeFormat, Text, binary, csv, text},
        transaction::{Transaction, TransactionStatus, TransactionType, ValidationError},
        writer::TransactionWriter,
    };
}
