//! Разбор TCP-протокола рукопожатия для команды STREAM.

use std::{collections::HashSet, net::SocketAddr};

use quote_common::ProtocolError;

/// Разобранная команда STREAM от клиента.
#[derive(Debug, Clone)]
pub struct StreamCommand {
    /// UDP-адрес клиента, на который нужно отправлять котировки.
    pub udp_addr: SocketAddr,
    /// Список тикеров, которые клиент хочет получать.
    pub tickers: Vec<String>,
}

/// Парсит строку TCP-команды в [`StreamCommand`].
///
/// Ожидаемый формат: `STREAM udp://HOST:PORT TICKER1,TICKER2,...\n`
///
/// Дублирующиеся тикеры удаляются с сохранением исходного порядка.
///
/// # Ошибки
///
/// Возвращает [`ProtocolError`], если формат невалиден или тикеры неизвестны.
///
/// # Примеры
///
/// ```
/// use std::collections::HashSet;
///
/// use quote_server::protocol::parse_command;
///
/// let known: HashSet<String> = ["AAPL", "TSLA"].iter().map(|s| s.to_string()).collect();
///
/// let cmd = parse_command("STREAM udp://127.0.0.1:5000 AAPL,TSLA\n", &known).unwrap();
/// assert_eq!(cmd.tickers, vec!["AAPL", "TSLA"]);
///
/// // Дубликаты удаляются с сохранением порядка
/// let cmd = parse_command("STREAM udp://127.0.0.1:5000 TSLA,AAPL,TSLA\n", &known).unwrap();
/// assert_eq!(cmd.tickers, vec!["TSLA", "AAPL"]);
///
/// // Неизвестные тикеры отклоняются
/// assert!(parse_command("STREAM udp://127.0.0.1:5000 FAKE\n", &known).is_err());
/// ```
pub fn parse_command(
    line: &str,
    known_tickers: &HashSet<String>,
) -> Result<StreamCommand, ProtocolError> {
    let line = line.trim();
    let parts: Vec<&str> = line.splitn(3, ' ').collect();

    if parts.len() != 3 || parts[0] != quote_common::CMD_STREAM {
        return Err(ProtocolError::InvalidCommand(format!(
            "expected '{} udp://HOST:PORT TICKER,...', got: {line}",
            quote_common::CMD_STREAM,
        )));
    }

    // Разбор udp://HOST:PORT
    let addr_str = parts[1]
        .strip_prefix("udp://")
        .ok_or_else(|| ProtocolError::InvalidAddress(parts[1].to_string()))?;

    let udp_addr: SocketAddr =
        addr_str.parse().map_err(|_| ProtocolError::InvalidAddress(addr_str.to_string()))?;

    // Разбор тикеров через запятую с дедупликацией и сохранением порядка
    let mut seen = HashSet::new();
    let tickers: Vec<String> = parts[2]
        .split(',')
        .map(|t| t.trim().to_uppercase())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .collect();

    if tickers.is_empty() {
        return Err(ProtocolError::InvalidCommand("no tickers specified".into()));
    }

    // Валидация по списку известных тикеров
    for ticker in &tickers {
        if !known_tickers.contains(ticker) {
            return Err(ProtocolError::UnknownTicker(ticker.clone()));
        }
    }

    Ok(StreamCommand { udp_addr, tickers })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> HashSet<String> {
        ["AAPL", "TSLA", "MSFT", "GOOGL"].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_valid_command() {
        let cmd = parse_command("STREAM udp://127.0.0.1:34254 AAPL,TSLA\n", &known()).unwrap();
        assert_eq!(cmd.udp_addr, "127.0.0.1:34254".parse().unwrap());
        assert_eq!(cmd.tickers, vec!["AAPL", "TSLA"]);
    }

    #[test]
    fn parse_single_ticker() {
        let cmd = parse_command("STREAM udp://0.0.0.0:5000 MSFT", &known()).unwrap();
        assert_eq!(cmd.tickers, vec!["MSFT"]);
    }

    #[test]
    fn parse_case_insensitive_tickers() {
        let cmd = parse_command("STREAM udp://127.0.0.1:5000 aapl,tsla", &known()).unwrap();
        assert_eq!(cmd.tickers, vec!["AAPL", "TSLA"]);
    }

    #[test]
    fn err_on_missing_prefix() {
        let result = parse_command("SUBSCRIBE udp://127.0.0.1:5000 AAPL", &known());
        assert!(result.is_err());
    }

    #[test]
    fn err_on_bad_address() {
        let result = parse_command("STREAM tcp://127.0.0.1:5000 AAPL", &known());
        assert!(result.is_err());
    }

    #[test]
    fn err_on_unknown_ticker() {
        let result = parse_command("STREAM udp://127.0.0.1:5000 AAPL,FAKE", &known());
        assert!(matches!(result, Err(ProtocolError::UnknownTicker(t)) if t == "FAKE"));
    }

    #[test]
    fn err_on_empty_tickers() {
        let result = parse_command("STREAM udp://127.0.0.1:5000 ", &known());
        assert!(result.is_err());
    }

    #[test]
    fn err_on_incomplete_command() {
        let result = parse_command("STREAM udp://127.0.0.1:5000", &known());
        assert!(result.is_err());
    }

    #[test]
    fn dedup_duplicate_tickers() {
        let cmd = parse_command("STREAM udp://127.0.0.1:5000 AAPL,TSLA,AAPL", &known()).unwrap();
        assert_eq!(cmd.tickers, vec!["AAPL", "TSLA"]);
    }
}
