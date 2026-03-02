//! Централизованный приём UDP PING и реестр последних отметок времени.
//!
//! Один поток ([`spawn_ping_receiver`]) читает все входящие UDP-датаграммы и
//! обновляет [`PingRegistry`], который потоки клиентов опрашивают через
//! [`last_ping()`](PingRegistry::last_ping).

use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use quote_common::PING_PAYLOAD;
use tracing::{debug, warn};

/// Потокобезопасный реестр PING-отметок для подключённых клиентов.
///
/// # Примеры
///
/// ```
/// use std::net::SocketAddr;
///
/// use quote_server::ping_registry::PingRegistry;
///
/// let reg = PingRegistry::new();
/// let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
///
/// reg.register(addr);
/// assert!(reg.last_ping(addr).is_some());
///
/// reg.remove(addr);
/// assert!(reg.last_ping(addr).is_none());
/// ```
pub struct PingRegistry {
    pings: Mutex<HashMap<SocketAddr, Instant>>,
}

impl Default for PingRegistry {
    fn default() -> Self {
        Self { pings: Mutex::new(HashMap::new()) }
    }
}

impl PingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Регистрирует клиента с начальной отметкой `Instant::now()`.
    pub fn register(&self, addr: SocketAddr) {
        self.pings.lock().unwrap().insert(addr, Instant::now());
    }

    /// Обновляет отметку времени для зарегистрированного адреса.
    /// Незарегистрированные адреса молча игнорируются.
    pub fn update(&self, addr: SocketAddr) {
        let mut map = self.pings.lock().unwrap();
        if let Some(ts) = map.get_mut(&addr) {
            *ts = Instant::now();
        }
    }

    /// Возвращает время последнего PING для данного адреса, или `None`.
    pub fn last_ping(&self, addr: SocketAddr) -> Option<Instant> {
        self.pings.lock().unwrap().get(&addr).copied()
    }

    /// Удаляет клиента из реестра.
    pub fn remove(&self, addr: SocketAddr) {
        self.pings.lock().unwrap().remove(&addr);
    }
}

/// Запускает фоновый поток, читающий все UDP-датаграммы и обновляющий реестр.
pub fn spawn_ping_receiver(
    socket: Arc<UdpSocket>,
    registry: Arc<PingRegistry>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        run_ping_receiver(&socket, &registry);
    })
}

fn run_ping_receiver(socket: &UdpSocket, registry: &PingRegistry) {
    let mut buf = [0u8; 64];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((n, peer)) => {
                if n == PING_PAYLOAD.len() && buf[..n] == *PING_PAYLOAD {
                    debug!(%peer, "got PING");
                    registry.update(peer);
                } else {
                    warn!(%peer, n, "unexpected UDP payload");
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => warn!(%e, "ping recv_from error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_sets_initial_timestamp() {
        let reg = PingRegistry::new();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();

        let before = Instant::now();
        reg.register(addr);
        let after = Instant::now();

        let ts = reg.last_ping(addr).expect("should be registered");
        assert!(ts >= before && ts <= after);
    }

    #[test]
    fn update_refreshes_timestamp() {
        let reg = PingRegistry::new();
        let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();

        reg.register(addr);
        let first = reg.last_ping(addr).unwrap();

        std::thread::sleep(Duration::from_millis(10));
        reg.update(addr);
        let second = reg.last_ping(addr).unwrap();

        assert!(second > first);
    }

    #[test]
    fn update_ignores_unknown_address() {
        let reg = PingRegistry::new();
        let addr: SocketAddr = "127.0.0.1:9002".parse().unwrap();

        // Не паникует, молча игнорирует
        reg.update(addr);
        assert!(reg.last_ping(addr).is_none());
    }

    #[test]
    fn remove_cleans_up() {
        let reg = PingRegistry::new();
        let addr: SocketAddr = "127.0.0.1:9003".parse().unwrap();

        reg.register(addr);
        assert!(reg.last_ping(addr).is_some());

        reg.remove(addr);
        assert!(reg.last_ping(addr).is_none());
    }

    #[test]
    fn last_ping_returns_none_for_unregistered() {
        let reg = PingRegistry::new();
        let addr: SocketAddr = "127.0.0.1:9004".parse().unwrap();

        assert!(reg.last_ping(addr).is_none());
    }

    #[test]
    fn multiple_clients_independent() {
        let reg = PingRegistry::new();
        let a1: SocketAddr = "127.0.0.1:9010".parse().unwrap();
        let a2: SocketAddr = "127.0.0.1:9011".parse().unwrap();

        reg.register(a1);
        reg.register(a2);

        std::thread::sleep(Duration::from_millis(10));
        reg.update(a1);

        let t1 = reg.last_ping(a1).unwrap();
        let t2 = reg.last_ping(a2).unwrap();
        assert!(t1 > t2, "a1 should have a newer timestamp after update");

        reg.remove(a1);
        assert!(reg.last_ping(a1).is_none());
        assert!(reg.last_ping(a2).is_some(), "a2 should be unaffected");
    }
}
