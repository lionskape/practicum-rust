//! Основные типы и структуры транзакций.

/// Тип банковской транзакции.
///
/// Определяет направление движения средств:
/// - [`Deposit`][TransactionType::Deposit]: средства поступают в систему (from_user_id = 0)
/// - [`Transfer`][TransactionType::Transfer]: средства перемещаются между пользователями
/// - [`Withdrawal`][TransactionType::Withdrawal]: средства выводятся из системы (to_user_id = 0)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionType {
    /// Внешнее пополнение счёта.
    /// Поле `from_user_id` должно быть равно `0` для этого типа.
    Deposit,
    /// Перевод между двумя пользователями внутри системы.
    Transfer,
    /// Вывод средств из системы.
    /// Поле `to_user_id` должно быть равно `0` для этого типа.
    Withdrawal,
}

/// Статус обработки транзакции.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionStatus {
    /// Транзакция успешно завершена.
    Success,
    /// Транзакция не была обработана.
    Failure,
    /// Транзакция ожидает обработки.
    Pending,
}

/// Запись банковской транзакции.
///
/// Эта структура представляет одну транзакцию в системе YPBank,
/// поддерживая все три формата: Text, Binary и CSV.
///
/// # Пример
///
/// ```
/// use parser::transaction::{Transaction, TransactionStatus, TransactionType};
///
/// let tx = Transaction {
///     tx_id: 1234567890123456,
///     tx_type: TransactionType::Deposit,
///     from_user_id: 0,
///     to_user_id: 9876543210987654,
///     amount: 10000,
///     timestamp: 1633036800000,
///     status: TransactionStatus::Success,
///     description: "Пополнение через терминал".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Уникальный идентификатор транзакции.
    pub tx_id: u64,
    /// Тип транзакции (Deposit, Transfer или Withdrawal).
    pub tx_type: TransactionType,
    /// ID счёта отправителя. Должен быть `0` для транзакций типа Deposit.
    pub from_user_id: u64,
    /// ID счёта получателя. Должен быть `0` для транзакций типа Withdrawal.
    pub to_user_id: u64,
    /// Сумма транзакции в наименьших единицах валюты (например, копейках).
    /// Положительное значение для зачислений, может быть отрицательным для списаний в бинарном
    /// формате.
    pub amount: i64,
    /// Unix-метка времени в миллисекундах, когда произошла транзакция.
    pub timestamp: u64,
    /// Текущий статус обработки транзакции.
    pub status: TransactionStatus,
    /// Человекочитаемое описание транзакции.
    pub description: String,
}
