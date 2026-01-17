//! Логика валидации транзакций.

use thiserror::Error;

use super::{Transaction, TransactionType};

/// Ошибки, возникающие при валидации транзакции.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("Неправильный источник для пополнения: {0} (from_user_id)")]
    InvalidDepositSource(u64),
    #[error("Неправильный назначение для вывода: {0} (to_user_id)")]
    InvalidWithdrawalDestination(u64),
    #[error("Перевод с одинаковым источником и назначением: {0}")]
    SelfTransfer(u64),
    #[error("Недопустимая сумма транзакции: {0} (ноль или отрицательная)")]
    InvalidAmount(i64),
}

impl Transaction {
    /// Проверяет транзакцию на соответствие бизнес-правилам YPBank.
    ///
    /// # Бизнес-правила
    ///
    /// - `Deposit`: `from_user_id` должен быть `0`
    /// - `Withdrawal`: `to_user_id` должен быть `0`
    /// - `Transfer`: `from_user_id` и `to_user_id` должны быть различными
    /// - Сумма должна быть положительной (ненулевой)
    ///
    /// # Возвращает
    ///
    /// `Ok(())` если все правила соблюдены, иначе `Err(ValidationError)`.
    ///
    /// # Пример
    ///
    /// ```
    /// use parser::transaction::{Transaction, TransactionStatus, TransactionType};
    ///
    /// let tx = Transaction {
    ///     tx_id: 1,
    ///     tx_type: TransactionType::Deposit,
    ///     from_user_id: 0, // корректно для пополнения
    ///     to_user_id: 123,
    ///     amount: 100,
    ///     timestamp: 1633036800000,
    ///     status: TransactionStatus::Success,
    ///     description: "Тест".to_string(),
    /// };
    /// assert!(tx.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self.tx_type {
            TransactionType::Deposit => {
                if self.from_user_id != 0 {
                    return Err(ValidationError::InvalidDepositSource(self.from_user_id));
                }
            }
            TransactionType::Transfer => {
                if self.from_user_id == self.to_user_id {
                    return Err(ValidationError::SelfTransfer(self.from_user_id));
                }
            }
            TransactionType::Withdrawal => {
                if self.to_user_id != 0 {
                    return Err(ValidationError::InvalidWithdrawalDestination(self.to_user_id));
                }
            }
        }

        if self.amount <= 0 {
            return Err(ValidationError::InvalidAmount(self.amount));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::TransactionStatus;

    /// Создаёт базовую транзакцию для тестов.
    /// Можно переопределить нужные поля после вызова.
    fn make_transaction(tx_type: TransactionType) -> Transaction {
        Transaction {
            tx_id: 1,
            tx_type,
            from_user_id: 100,
            to_user_id: 200,
            amount: 1000,
            timestamp: 1633036800000,
            status: TransactionStatus::Success,
            description: "Тестовая транзакция".to_string(),
        }
    }

    // ==================== Позитивные тесты ====================

    #[test]
    fn valid_deposit() {
        let mut tx = make_transaction(TransactionType::Deposit);
        tx.from_user_id = 0; // Для депозита источник должен быть 0
        assert!(tx.validate().is_ok());
    }

    #[test]
    fn valid_transfer() {
        let tx = make_transaction(TransactionType::Transfer);
        // from_user_id=100, to_user_id=200 — разные, всё ок
        assert!(tx.validate().is_ok());
    }

    #[test]
    fn valid_withdrawal() {
        let mut tx = make_transaction(TransactionType::Withdrawal);
        tx.to_user_id = 0; // Для вывода назначение должно быть 0
        assert!(tx.validate().is_ok());
    }

    // ==================== Негативные тесты ====================

    #[test]
    fn deposit_with_nonzero_source_fails() {
        let tx = make_transaction(TransactionType::Deposit);
        // from_user_id=100, но для депозита должен быть 0
        assert_eq!(tx.validate(), Err(ValidationError::InvalidDepositSource(100)));
    }

    #[test]
    fn withdrawal_with_nonzero_destination_fails() {
        let tx = make_transaction(TransactionType::Withdrawal);
        // to_user_id=200, но для вывода должен быть 0
        assert_eq!(tx.validate(), Err(ValidationError::InvalidWithdrawalDestination(200)));
    }

    #[test]
    fn self_transfer_fails() {
        let mut tx = make_transaction(TransactionType::Transfer);
        tx.from_user_id = 42;
        tx.to_user_id = 42; // Перевод самому себе
        assert_eq!(tx.validate(), Err(ValidationError::SelfTransfer(42)));
    }

    // ==================== Тесты на сумму ====================

    #[test]
    fn zero_amount_fails() {
        let mut tx = make_transaction(TransactionType::Transfer);
        tx.amount = 0;
        assert_eq!(tx.validate(), Err(ValidationError::InvalidAmount(0)));
    }

    #[test]
    fn negative_amount_fails() {
        let mut tx = make_transaction(TransactionType::Transfer);
        tx.amount = -500;
        assert_eq!(tx.validate(), Err(ValidationError::InvalidAmount(-500)));
    }

    #[test]
    fn minimal_positive_amount_passes() {
        let mut tx = make_transaction(TransactionType::Transfer);
        tx.amount = 1;
        assert_eq!(tx.validate(), Ok(()));
    }
}
