//! Установка TCP-соединения — отправка команды STREAM и разбор ответа сервера.

use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use quote_common::{CMD_STREAM, RESP_ERR, RESP_OK};
use tracing::info;

/// Результат успешного рукопожатия с сервером.
pub struct HandshakeResult {
    /// UDP-адрес сервера, на который клиент отправляет PING.
    pub server_udp_addr: SocketAddr,
}

/// Подключается к серверу котировок по TCP, отправляет команду STREAM и читает ответ.
///
/// # Аргументы
/// * `server_addr` — TCP-адрес сервера котировок (например, `127.0.0.1:8080`).
/// * `client_udp_addr` — наш локальный UDP-адрес, на который сервер будет слать котировки.
/// * `tickers` — список тикеров для подписки.
///
/// # Возвращает
/// [`HandshakeResult`] с UDP-адресом сервера в случае успеха.
pub fn handshake(
    server_addr: &str,
    client_udp_addr: SocketAddr,
    tickers: &[String],
) -> Result<HandshakeResult> {
    let sock_addr: SocketAddr =
        server_addr.parse().with_context(|| format!("parse server address: {server_addr}"))?;
    let mut stream = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(5))
        .with_context(|| format!("connect to {server_addr} (5 s timeout)"))?;
    info!(%server_addr, "TCP connected");

    // Отправка: STREAM udp://HOST:PORT TICKER1,TICKER2\n
    let ticker_list = tickers.join(",");
    let command = format!("{CMD_STREAM} udp://{client_udp_addr} {ticker_list}\n");
    stream.write_all(command.as_bytes()).context("send STREAM command")?;
    info!(%command, "sent command");

    // Чтение строки ответа
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).context("read server response")?;
    let response = response.trim();

    if let Some(addr_str) = response.strip_prefix(RESP_OK) {
        let addr_str = addr_str.trim();
        let server_udp_addr: SocketAddr =
            addr_str.parse().with_context(|| format!("parse server UDP addr: {addr_str}"))?;
        info!(%server_udp_addr, "handshake OK");
        Ok(HandshakeResult { server_udp_addr })
    } else if let Some(err_msg) = response.strip_prefix(RESP_ERR) {
        bail!("server rejected subscription: {}", err_msg.trim());
    } else {
        bail!("unexpected server response: {response}");
    }
}
