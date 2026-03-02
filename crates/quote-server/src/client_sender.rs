//! Поток UDP-отправки для каждого клиента — фильтрует котировки и отправляет их
//! как JSON-датаграммы.
//!
//! Проверяет PING-таймаут через [`PingRegistry`](crate::ping_registry::PingRegistry):
//! если клиент перестаёт отправлять PING дольше чем
//! [`PING_TIMEOUT_SECS`](quote_common::PING_TIMEOUT_SECS), поток завершается.

use std::{
    collections::HashSet,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::Duration,
};

use crossbeam_channel::Receiver;
use quote_common::{PING_TIMEOUT_SECS, StockQuote};
use tracing::{info, warn};

use crate::ping_registry::PingRegistry;

/// Реестр каналов рассылки для всех подключённых клиентов.
///
/// Поток генератора вызывает [`broadcast()`](ClientRegistry::broadcast) на каждом тике;
/// отвалившиеся каналы автоматически удаляются.
///
/// # Примеры
///
/// ```
/// use std::sync::Arc;
///
/// use quote_common::StockQuote;
/// use quote_server::client_sender::ClientRegistry;
///
/// let registry = ClientRegistry::new();
/// let rx = registry.subscribe();
///
/// let quotes = Arc::new(vec![StockQuote {
///     ticker: "AAPL".into(),
///     price: 150.0,
///     volume: 1000,
///     timestamp: 0,
/// }]);
/// registry.broadcast(quotes);
///
/// let received = rx.recv().unwrap();
/// assert_eq!(received[0].ticker, "AAPL");
/// ```
pub struct ClientRegistry {
    senders: Mutex<Vec<crossbeam_channel::Sender<Arc<Vec<StockQuote>>>>>,
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self { senders: Mutex::new(Vec::new()) }
    }
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Регистрирует новый клиентский канал и возвращает принимающую сторону.
    pub fn subscribe(&self) -> Receiver<Arc<Vec<StockQuote>>> {
        let (tx, rx) = crossbeam_channel::bounded(64);
        self.senders.lock().unwrap().push(tx);
        rx
    }

    /// Рассылает котировки всем живым клиентам; удаляет отключённых отправителей.
    pub fn broadcast(&self, quotes: Arc<Vec<StockQuote>>) {
        let mut senders = self.senders.lock().unwrap();
        senders.retain(|tx| tx.try_send(Arc::clone(&quotes)).is_ok());
    }
}

/// Запускает поток, который получает пакеты котировок из реестра, фильтрует по
/// подписке клиента, сериализует каждую котировку в JSON и отправляет
/// через UDP на адрес клиента.
///
/// PING-таймаут проверяется через [`PingRegistry`] — приём UDP-датаграмм
/// осуществляется отдельным потоком
/// ([`spawn_ping_receiver`](crate::ping_registry::spawn_ping_receiver)). Если PING не приходит в
/// течение [`PING_TIMEOUT_SECS`], поток завершается и клиент считается отключённым.
pub fn spawn_client_sender(
    server_socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    tickers: HashSet<String>,
    rx: Receiver<Arc<Vec<StockQuote>>>,
    ping_registry: Arc<PingRegistry>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        info!(%client_addr, "client sender thread started");
        run_client_sender(server_socket, client_addr, tickers, rx, &ping_registry);
        ping_registry.remove(client_addr);
        info!(%client_addr, "client sender thread exited");
    })
}

fn run_client_sender(
    socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    tickers: HashSet<String>,
    rx: Receiver<Arc<Vec<StockQuote>>>,
    ping_registry: &PingRegistry,
) {
    // Короткий таймаут для чередования отправки котировок и проверки PING
    let tick = Duration::from_millis(50);

    loop {
        // ── 1. Попытка получить котировки (неблокирующе, с коротким таймаутом) ──
        match rx.recv_timeout(tick) {
            Ok(quotes) => {
                for quote in quotes.iter().filter(|q| tickers.contains(&q.ticker)) {
                    match serde_json::to_vec(quote) {
                        Ok(data) => {
                            if let Err(e) = socket.send_to(&data, client_addr) {
                                warn!(%client_addr, %e, "failed to send quote");
                            }
                        }
                        Err(e) => warn!(%e, "failed to serialize quote"),
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => { /* штатно, проверяем ping */
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                info!(%client_addr, "broadcast channel closed, exiting");
                return;
            }
        }

        // ── 2. Проверка PING-таймаута через реестр ──
        if ping_registry
            .last_ping(client_addr)
            .is_none_or(|ts| ts.elapsed().as_secs() > PING_TIMEOUT_SECS)
        {
            warn!(%client_addr, "PING timeout, disconnecting client");
            return;
        }
    }
}
