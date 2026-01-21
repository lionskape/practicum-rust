//! Модуль ошибок парсинга транзакций.

use thiserror::Error;

use crate::transaction::ValidationError;

/// Главная ошибка парсинга транзакций.
///
/// Объединяет все возможные ошибки при работе с форматами YPBank:
/// I/O ошибки, ошибки парсинга каждого формата и ошибки валидации.
#[derive(Debug, Error)]
pub enum ParseError {
    // === I/O ошибки ===
    /// Ошибка ввода/вывода.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // === Ошибки Text формата ===
    /// Некорректное поле в текстовом формате.
    #[error("Invalid field '{field}' at line {line}: {message}")]
    InvalidField {
        /// Имя поля.
        field: String,
        /// Номер строки (1-based).
        line: usize,
        /// Описание ошибки.
        message: String,
    },

    /// Отсутствует обязательное поле.
    #[error("Missing required field '{0}'")]
    MissingField(String),

    /// Дублирование поля в записи.
    #[error("Duplicate field '{field}' at line {line}")]
    DuplicateField {
        /// Имя дублирующегося поля.
        field: String,
        /// Номер строки (1-based).
        line: usize,
    },

    /// Некорректное значение поля.
    #[error("Invalid value for {field}: expected {expected}, got '{actual}'")]
    InvalidValue {
        /// Имя поля.
        field: String,
        /// Ожидаемый тип/формат.
        expected: String,
        /// Фактическое значение.
        actual: String,
    },

    // === Ошибки Binary формата ===
    /// Некорректные magic bytes в бинарном формате.
    #[error("Invalid magic bytes: expected 'YPBN', got {0:?}")]
    InvalidMagic([u8; 4]),

    /// Несоответствие размера записи в заголовке и фактического.
    #[error("Record size mismatch: header says {expected}, but got {actual} bytes")]
    RecordSizeMismatch {
        /// Размер из заголовка.
        expected: u32,
        /// Фактический размер.
        actual: u32,
    },

    /// Некорректное значение для enum-поля (TX_TYPE или STATUS).
    #[error("Invalid enum value {value} for {field}")]
    InvalidEnumValue {
        /// Имя поля.
        field: String,
        /// Некорректное числовое значение.
        value: u8,
    },

    // === Ошибки UTF-8 ===
    /// Некорректная UTF-8 строка в описании.
    #[error("Invalid UTF-8 in description: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    // === Ошибки валидации ===
    /// Ошибка бизнес-валидации транзакции.
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    // === Прочее ===
    /// Неизвестный формат файла.
    #[error("Unknown format")]
    UnknownFormat,

    /// Неожиданный конец файла.
    #[error("Unexpected end of file")]
    UnexpectedEof,
}

/// Удобный alias для Result с ParseError.
pub type ParseResult<T> = Result<T, ParseError>;

// === Конверсия из serde::Error ===
impl From<crate::serde::Error> for ParseError {
    fn from(err: crate::serde::Error) -> Self {
        use crate::serde::Error as SerdeErr;
        match err {
            SerdeErr::Message(msg) => {
                Self::InvalidField { field: "unknown".to_string(), line: 0, message: msg }
            }
            SerdeErr::Io(e) => Self::Io(e),
            SerdeErr::InvalidUtf8(e) => Self::InvalidUtf8(e),
            SerdeErr::UnexpectedEof => Self::UnexpectedEof,
            SerdeErr::InvalidMagic(magic) => Self::InvalidMagic(magic),
            SerdeErr::InvalidEnumValue { field, value } => {
                Self::InvalidEnumValue { field: field.to_string(), value }
            }
            SerdeErr::RecordSizeMismatch { expected, actual } => {
                Self::RecordSizeMismatch { expected, actual }
            }
            SerdeErr::MissingField(f) => Self::MissingField(f),
            SerdeErr::InvalidFieldFormat(msg) => {
                Self::InvalidField { field: "unknown".to_string(), line: 0, message: msg }
            }
            SerdeErr::ExpectedStruct => Self::InvalidField {
                field: "root".to_string(),
                line: 0,
                message: "expected struct".to_string(),
            },
            SerdeErr::UnknownField(f) => {
                Self::InvalidField { field: f, line: 0, message: "unknown field".to_string() }
            }
            SerdeErr::UnsupportedType(ty) => Self::InvalidField {
                field: "unknown".to_string(),
                line: 0,
                message: format!("unsupported type: {ty}"),
            },
            SerdeErr::TrailingData => Self::InvalidField {
                field: "unknown".to_string(),
                line: 0,
                message: "trailing data after record".to_string(),
            },
        }
    }
}
