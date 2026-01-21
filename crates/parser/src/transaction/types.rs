//! Основные типы и структуры транзакций.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// Тип банковской транзакции.
///
/// Определяет направление движения средств:
/// - [`Deposit`][TransactionType::Deposit]: средства поступают в систему (from_user_id = 0)
/// - [`Transfer`][TransactionType::Transfer]: средства перемещаются между пользователями
/// - [`Withdrawal`][TransactionType::Withdrawal]: средства выводятся из системы (to_user_id = 0)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// Внешнее пополнение счёта.
    /// Поле `from_user_id` должно быть равно `0` для этого типа.
    #[serde(rename = "DEPOSIT")]
    Deposit,
    /// Перевод между двумя пользователями внутри системы.
    #[serde(rename = "TRANSFER")]
    Transfer,
    /// Вывод средств из системы.
    /// Поле `to_user_id` должно быть равно `0` для этого типа.
    #[serde(rename = "WITHDRAWAL")]
    Withdrawal,
}

impl TransactionType {
    /// Возвращает строковое представление типа транзакции.
    ///
    /// # Пример
    /// ```
    /// use parser::transaction::TransactionType;
    /// assert_eq!(TransactionType::Deposit.as_str(), "DEPOSIT");
    /// ```
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "DEPOSIT",
            Self::Transfer => "TRANSFER",
            Self::Withdrawal => "WITHDRAWAL",
        }
    }
}

impl FromStr for TransactionType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DEPOSIT" => Ok(Self::Deposit),
            "TRANSFER" => Ok(Self::Transfer),
            "WITHDRAWAL" => Ok(Self::Withdrawal),
            _ => Err(ParseError::InvalidValue {
                field: "TX_TYPE".to_string(),
                expected: "DEPOSIT, TRANSFER, or WITHDRAWAL".to_string(),
                actual: s.to_string(),
            }),
        }
    }
}

impl TryFrom<u8> for TransactionType {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Deposit),
            1 => Ok(Self::Transfer),
            2 => Ok(Self::Withdrawal),
            v => Err(ParseError::InvalidEnumValue { field: "TX_TYPE".to_string(), value: v }),
        }
    }
}

impl From<TransactionType> for u8 {
    fn from(t: TransactionType) -> Self {
        match t {
            TransactionType::Deposit => 0,
            TransactionType::Transfer => 1,
            TransactionType::Withdrawal => 2,
        }
    }
}

/// Статус обработки транзакции.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Транзакция успешно завершена.
    #[serde(rename = "SUCCESS")]
    Success,
    /// Транзакция не была обработана.
    #[serde(rename = "FAILURE")]
    Failure,
    /// Транзакция ожидает обработки.
    #[serde(rename = "PENDING")]
    Pending,
}

impl TransactionStatus {
    /// Возвращает строковое представление статуса.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::Failure => "FAILURE",
            Self::Pending => "PENDING",
        }
    }
}

impl FromStr for TransactionStatus {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SUCCESS" => Ok(Self::Success),
            "FAILURE" => Ok(Self::Failure),
            "PENDING" => Ok(Self::Pending),
            _ => Err(ParseError::InvalidValue {
                field: "STATUS".to_string(),
                expected: "SUCCESS, FAILURE, or PENDING".to_string(),
                actual: s.to_string(),
            }),
        }
    }
}

impl TryFrom<u8> for TransactionStatus {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::Failure),
            2 => Ok(Self::Pending),
            v => Err(ParseError::InvalidEnumValue { field: "STATUS".to_string(), value: v }),
        }
    }
}

impl From<TransactionStatus> for u8 {
    fn from(s: TransactionStatus) -> Self {
        match s {
            TransactionStatus::Success => 0,
            TransactionStatus::Failure => 1,
            TransactionStatus::Pending => 2,
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Уникальный идентификатор транзакции.
    #[serde(rename = "TX_ID")]
    pub tx_id: u64,
    /// Тип транзакции (Deposit, Transfer или Withdrawal).
    #[serde(rename = "TX_TYPE")]
    pub tx_type: TransactionType,
    /// ID счёта отправителя. Должен быть `0` для транзакций типа Deposit.
    #[serde(rename = "FROM_USER_ID")]
    pub from_user_id: u64,
    /// ID счёта получателя. Должен быть `0` для транзакций типа Withdrawal.
    #[serde(rename = "TO_USER_ID")]
    pub to_user_id: u64,
    /// Сумма транзакции в наименьших единицах валюты (например, копейках).
    /// Положительное значение для зачислений, может быть отрицательным для списаний в бинарном
    /// формате.
    #[serde(rename = "AMOUNT")]
    pub amount: i64,
    /// Unix-метка времени в миллисекундах, когда произошла транзакция.
    #[serde(rename = "TIMESTAMP")]
    pub timestamp: u64,
    /// Текущий статус обработки транзакции.
    #[serde(rename = "STATUS")]
    pub status: TransactionStatus,
    /// Человекочитаемое описание транзакции.
    #[serde(rename = "DESCRIPTION")]
    pub description: String,
}
