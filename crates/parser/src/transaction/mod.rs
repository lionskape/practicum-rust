//! Модель данных транзакций для форматов YPBank.
//!
//! Этот модуль определяет основную структуру [`Transaction`] и связанные типы,
//! общие для всех форматов YPBank (Text, Binary, CSV).

mod types;
mod validation;

pub use types::{Transaction, TransactionStatus, TransactionType};
pub use validation::ValidationError;
