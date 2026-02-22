//! Quote-server — генерирует синтетические котировки и передаёт их клиентам по UDP.
//!
//! # Запуск
//!
//! ```bash
//! RUST_LOG=info cargo run -p quote-server -- --tcp-addr 127.0.0.1:8080
//! ```

use std::{
    collections::HashSet,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, UdpSocket},
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use quote_common::{GENERATION_INTERVAL_MS, RESP_ERR, RESP_OK};
use quote_server::{
    client_sender::{ClientRegistry, spawn_client_sender},
    generator::QuoteGenerator,
    protocol::parse_command,
};
use tracing::{error, info, warn};

/// Сервер потоковых котировок.
///
/// Генерирует синтетические котировки с частотой 10 Гц и отправляет их
/// подписанным клиентам по UDP. Клиенты подключаются по TCP для подписки.
#[derive(Parser, Debug)]
#[command(name = "quote-server")]
#[command(version, about)]
struct Args {
    /// TCP-адрес для приёма клиентских подключений.
    #[arg(long, default_value = "127.0.0.1:8080")]
    tcp_addr: String,

    /// UDP-адрес для отправки котировок и приёма PING.
    #[arg(long, default_value = "0.0.0.0:0")]
    udp_addr: String,
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

    // Загрузка известных тикеров
    let all_tickers = quote_server::all_tickers();
    let known_set: HashSet<String> = all_tickers.iter().cloned().collect();
    info!(count = all_tickers.len(), "loaded tickers");

    // Привязка UDP-сокета для отправки котировок / приёма PING
    let udp_socket =
        UdpSocket::bind(&args.udp_addr).with_context(|| format!("bind UDP {}", args.udp_addr))?;
    udp_socket.set_nonblocking(true).context("set UDP non-blocking")?;
    let mut udp_local_addr = udp_socket.local_addr().context("get UDP local addr")?;
    // Если UDP привязан к 0.0.0.0 (wildcard), подставляем IP из TCP-адреса,
    // иначе клиент получит немаршрутизируемый адрес назначения для PING.
    if udp_local_addr.ip().is_unspecified()
        && let Ok(tcp_addr) = args.tcp_addr.parse::<std::net::SocketAddr>()
    {
        udp_local_addr.set_ip(tcp_addr.ip());
    }
    info!(%udp_local_addr, "UDP socket ready");
    let udp_socket = Arc::new(udp_socket);

    // Реестр клиентов (разделяется между потоком генератора и TCP-акцептором)
    let registry = Arc::new(ClientRegistry::new());

    // ── Запуск потока генератора ──
    let gen_registry = Arc::clone(&registry);
    thread::spawn(move || {
        let mut generator = QuoteGenerator::new(all_tickers);
        let interval = Duration::from_millis(GENERATION_INTERVAL_MS);
        loop {
            let quotes = Arc::new(generator.generate_all());
            gen_registry.broadcast(quotes);
            thread::sleep(interval);
        }
    });

    // ── TCP-слушатель — приём клиентов ──
    let listener =
        TcpListener::bind(&args.tcp_addr).with_context(|| format!("bind TCP {}", args.tcp_addr))?;
    info!(addr = %args.tcp_addr, "TCP listener started");

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                error!(%e, "accept failed");
                continue;
            }
        };

        let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "unknown".into());
        info!(%peer, "new TCP connection");

        // Чтение одной строки (команда STREAM)
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        if let Err(e) = reader.read_line(&mut line) {
            error!(%peer, %e, "failed to read command");
            continue;
        }
        let mut stream = reader.into_inner();

        // Разбор команды
        match parse_command(&line, &known_set) {
            Ok(cmd) => {
                // Ответ OK с UDP-адресом сервера
                let response = format!("{RESP_OK} {udp_local_addr}\n");
                if let Err(e) = stream.write_all(response.as_bytes()) {
                    error!(%peer, %e, "failed to send OK");
                    continue;
                }

                info!(%peer, tickers = ?cmd.tickers, udp = %cmd.udp_addr, "client subscribed");

                // Подписка и запуск потока отправки
                let rx = registry.subscribe();
                let ticker_set: HashSet<String> = cmd.tickers.into_iter().collect();
                spawn_client_sender(Arc::clone(&udp_socket), cmd.udp_addr, ticker_set, rx);
            }
            Err(e) => {
                let response = format!("{RESP_ERR} {e}\n");
                if let Err(write_err) = stream.write_all(response.as_bytes()) {
                    warn!(%peer, %write_err, "failed to send ERR response");
                }
                error!(%peer, %e, "rejected client");
            }
        }
    }

    Ok(())
}
