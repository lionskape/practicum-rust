//! Отправка PING — периодически шлёт PING-пакеты серверу по UDP.

use std::{
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use quote_common::{PING_INTERVAL_SECS, PING_PAYLOAD};
use tracing::{debug, warn};

/// Запускает поток, отправляющий PING-пакеты с фиксированным интервалом.
///
/// Поток работает, пока `shutdown` не будет установлен в `true`.
pub fn spawn_ping_thread(
    socket: Arc<UdpSocket>,
    server_udp_addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let interval = Duration::from_secs(PING_INTERVAL_SECS);
        while !shutdown.load(Ordering::Acquire) {
            match socket.send_to(PING_PAYLOAD, server_udp_addr) {
                Ok(_) => debug!("PING sent"),
                Err(e) => warn!(%e, "failed to send PING"),
            }
            thread::sleep(interval);
        }
        debug!("ping thread exiting");
    })
}
