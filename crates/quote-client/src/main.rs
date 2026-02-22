//! Quote-client — подключается к серверу котировок и отображает потоковые котировки.
//!
//! # Запуск
//!
//! ```bash
//! RUST_LOG=info cargo run -p quote-client -- \
//!     --server-addr 127.0.0.1:8080 \
//!     --udp-port 34254 \
//!     --tickers-file tickers.txt
//! ```

use std::{
    fs,
    net::UdpSocket,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Context, Result, ensure};
use clap::Parser;
use quote_client::{connection::handshake, ping::spawn_ping_thread, receiver::run_receive_loop};
use tracing::{error, info};

/// Клиент потоковых котировок.
///
/// Подключается к серверу котировок, подписывается на тикеры и отображает
/// получаемые котировки в реальном времени.
#[derive(Parser, Debug)]
#[command(name = "quote-client")]
#[command(version, about)]
struct Args {
    /// TCP-адрес сервера котировок.
    #[arg(long)]
    server_addr: String,

    /// Локальный UDP-порт для приёма котировок.
    #[arg(long)]
    udp_port: u16,

    /// Путь к файлу с тикерами (по одному на строку).
    #[arg(long)]
    tickers_file: PathBuf,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = run() {
        error!("{e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Чтение тикеров из файла
    let tickers_raw = fs::read_to_string(&args.tickers_file)
        .with_context(|| format!("read tickers file: {}", args.tickers_file.display()))?;
    let tickers: Vec<String> = tickers_raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.to_uppercase())
        .collect();
    info!(count = tickers.len(), "loaded tickers");
    ensure!(!tickers.is_empty(), "tickers file is empty: {}", args.tickers_file.display());

    // Привязка локального UDP-сокета к тому же интерфейсу, что и сервер.
    // `0.0.0.0` нельзя использовать как адрес назначения (No route to host),
    // поэтому берём IP из адреса сервера.
    let server_ip = args
        .server_addr
        .parse::<std::net::SocketAddr>()
        .map(|a| a.ip())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let udp_bind_addr = format!("{server_ip}:{}", args.udp_port);
    let udp_socket =
        UdpSocket::bind(&udp_bind_addr).with_context(|| format!("bind UDP {udp_bind_addr}"))?;
    let local_udp_addr = udp_socket.local_addr().context("get local UDP addr")?;
    info!(%local_udp_addr, "UDP socket bound");
    let udp_socket = Arc::new(udp_socket);

    // TCP-рукопожатие — подписка на тикеры
    let result = handshake(&args.server_addr, local_udp_addr, &tickers)?;
    info!(server_udp = %result.server_udp_addr, "subscribed successfully");

    // Флаг завершения, разделяемый между потоками
    let shutdown = Arc::new(AtomicBool::new(false));

    // Регистрация обработчика Ctrl+C
    let shutdown_ctrlc = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        info!("Ctrl+C received, shutting down...");
        shutdown_ctrlc.store(true, Ordering::Release);
    })
    .context("set Ctrl+C handler")?;

    // Запуск PING-потока
    let ping_handle =
        spawn_ping_thread(Arc::clone(&udp_socket), result.server_udp_addr, Arc::clone(&shutdown));

    // Цикл приёма котировок на главном потоке (блокируется до завершения)
    run_receive_loop(Arc::clone(&udp_socket), Arc::clone(&shutdown));

    // Ожидание завершения PING-потока
    if let Err(e) = ping_handle.join() {
        error!("ping thread panicked: {e:?}");
    }
    info!("client shut down cleanly");

    Ok(())
}
