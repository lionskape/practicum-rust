//! Поток UDP-отправки для каждого клиента — фильтрует котировки и отправляет их
//! как JSON-датаграммы.
//!
//! Также отслеживает PING: если клиент перестаёт отправлять PING дольше чем
//! [`PING_TIMEOUT_SECS`](quote_common::PING_TIMEOUT_SECS), поток завершается.

use std::{
    collections::HashSet,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crossbeam_channel::Receiver;
use quote_common::{PING_PAYLOAD, PING_TIMEOUT_SECS, StockQuote};
use tracing::{debug, info, warn};

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
/// Поток также слушает PING-пакеты от клиента на том же UDP-сокете.
/// Если PING не приходит в течение [`PING_TIMEOUT_SECS`], поток завершается
/// и клиент считается отключённым.
pub fn spawn_client_sender(
    server_socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    tickers: HashSet<String>,
    rx: Receiver<Arc<Vec<StockQuote>>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        info!(%client_addr, "client sender thread started");
        run_client_sender(server_socket, client_addr, tickers, rx);
        info!(%client_addr, "client sender thread exited");
    })
}

fn run_client_sender(
    socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    tickers: HashSet<String>,
    rx: Receiver<Arc<Vec<StockQuote>>>,
) {
    let mut last_ping = Instant::now();

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

        // ── 2. Проверка входящего PING от клиента ──
        let mut ping_buf = [0u8; 64];
        match socket.recv_from(&mut ping_buf) {
            Ok((n, peer)) if peer == client_addr => {
                if n == PING_PAYLOAD.len() && &ping_buf[..n] == PING_PAYLOAD.as_slice() {
                    debug!(%client_addr, "got PING");
                    last_ping = Instant::now();
                } else {
                    warn!(%peer, n, "unexpected payload from client");
                }
            }
            Ok(_) => { /* пакет с другого адреса — игнорируем */ }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => { /* нет данных */
            }
            Err(e) => warn!(%e, "recv_from error"),
        }

        // Проверка таймаута вне зависимости от результата recv_from
        if last_ping.elapsed().as_secs() > PING_TIMEOUT_SECS {
            warn!(%client_addr, "PING timeout, disconnecting client");
            return;
        }
    }
}
