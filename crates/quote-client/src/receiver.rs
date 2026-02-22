//! UDP-приёмник — читает JSON-котировки от сервера и логирует их.

use std::{
    net::UdpSocket,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use quote_common::{StockQuote, UDP_BUF_SIZE};
use tracing::{debug, info, warn};

/// Запускает цикл приёма: читает UDP-датаграммы, десериализует JSON-котировки и логирует их.
///
/// Завершается, когда `shutdown` устанавливается в `true`.
pub fn run_receive_loop(socket: Arc<UdpSocket>, shutdown: Arc<AtomicBool>) {
    // Короткий таймаут чтения для периодической проверки флага завершения
    socket.set_read_timeout(Some(Duration::from_millis(500))).ok();

    let mut buf = [0u8; UDP_BUF_SIZE];

    while !shutdown.load(Ordering::Acquire) {
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                let data = &buf[..n];
                match serde_json::from_slice::<StockQuote>(data) {
                    Ok(quote) => {
                        info!(
                            ticker = %quote.ticker,
                            price = format_args!("{:.2}", quote.price),
                            volume = quote.volume,
                            "quote received"
                        );
                    }
                    Err(e) => {
                        debug!(%src, %e, "non-quote datagram (ignoring)");
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Таймаут — возвращаемся к проверке флага завершения
                continue;
            }
            Err(e) => {
                warn!(%e, "UDP recv error");
            }
        }
    }

    debug!("receive loop exiting");
}
