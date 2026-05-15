use std::{env, str::FromStr, sync::Arc, time::Duration};

use anchor_lang::{AnchorDeserialize, Discriminator, InstructionData, ToAccountMetas};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use dotenvy::dotenv;
use futures::StreamExt;
use regex::Regex;
use serde::Serialize;
use sol_usd_oracle::PRICE_DECIMALS;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_response::RpcLogsResponse,
};
use solana_rpc_client_types::config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature, Signer},
    transaction::Transaction,
};
use tokio::time::interval;
use tracing::{error, info, warn};

const DEFAULT_PRICE_POLL_INTERVAL_SEC: u64 = 600; // 10 minutes; live price from Binance when MOCK_PRICE is not set

#[derive(Clone)]
struct Config {
    rpc_http: String,
    rpc_ws: String,
    oracle_program_id: Pubkey,
    oracle_state: Pubkey,
    minter_program_id: Pubkey,
    backend_keypair_path: String,
    price_poll_interval: Duration,
    mock_price: Option<u64>,
    price_api_url: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        let rpc_http =
            env::var("SOLANA_RPC_HTTP").context("SOLANA_RPC_HTTP env var is required")?;
        let rpc_ws = env::var("SOLANA_RPC_WS").context("SOLANA_RPC_WS env var is required")?;
        let oracle_program_id = Pubkey::from_str(
            &env::var("ORACLE_PROGRAM_ID").context("ORACLE_PROGRAM_ID is required")?,
        )?;
        let oracle_state =
            Pubkey::from_str(&env::var("ORACLE_STATE_PUBKEY").context("ORACLE_STATE_PUBKEY")?)?;
        let minter_program_id = Pubkey::from_str(
            &env::var("MINTER_PROGRAM_ID").context("MINTER_PROGRAM_ID is required")?,
        )?;
        let mut backend_keypair_path =
            env::var("BACKEND_KEYPAIR_PATH").context("BACKEND_KEYPAIR_PATH is required")?;
        if backend_keypair_path.starts_with("~/") {
            if let Some(home) = env::var_os("HOME") {
                backend_keypair_path =
                    format!("{}/{}", home.to_string_lossy(), &backend_keypair_path[2..]);
            }
        }
        let poll = env::var("PRICE_POLL_INTERVAL_SEC")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_PRICE_POLL_INTERVAL_SEC);
        let mock_price = env::var("MOCK_PRICE").ok().and_then(|s| s.parse::<u64>().ok());
        let price_api_url = env::var("PRICE_API_URL").ok();

        Ok(Self {
            rpc_http,
            rpc_ws,
            oracle_program_id,
            oracle_state,
            minter_program_id,
            backend_keypair_path,
            price_poll_interval: Duration::from_secs(poll),
            mock_price,
            price_api_url,
        })
    }
}

#[derive(Clone)]
enum PriceSource {
    Mock(u64),
    Http { url: String },
}

impl PriceSource {
    fn from_config(cfg: &Config) -> Self {
        if let Some(mock) = cfg.mock_price {
            PriceSource::Mock(mock)
        } else if let Some(url) = cfg.price_api_url.clone() {
            PriceSource::Http { url }
        } else {
            PriceSource::Http {
                url: "https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT".to_string(),
            }
        }
    }

    async fn fetch_price(&self) -> Result<u64> {
        match self {
            PriceSource::Mock(val) => Ok(*val),
            PriceSource::Http { url } => {
                #[derive(serde::Deserialize)]
                struct Resp {
                    price: String,
                }
                let resp: Resp = reqwest::get(url).await?.json().await?;
                to_fixed_6(&resp.price)
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct TokenCreatedLog {
    creator: String,
    mint: String,
    decimals: u8,
    initial_supply: u64,
    fee_lamports: u64,
    sol_usd_price: u64,
    slot: u64,
    signature: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = Config::from_env()?;
    let price_source = PriceSource::from_config(&cfg);
    let admin = Arc::new(
        read_keypair_file(&cfg.backend_keypair_path)
            .map_err(|e| anyhow!(e.to_string()))
            .context("read backend keypair")?,
    );

    let price_task = tokio::spawn(run_price_updater(cfg.clone(), price_source, admin.clone()));
    let listener_task = tokio::spawn(run_event_listener(cfg.clone()));

    let (price_res, listener_res) = tokio::try_join!(price_task, listener_task)?;
    price_res?;
    listener_res?;
    Ok(())
}

async fn run_price_updater(
    cfg: Config,
    price_source: PriceSource,
    admin: Arc<Keypair>,
) -> Result<()> {
    let client = RpcClient::new(cfg.rpc_http.clone());
    let mut ticker = interval(cfg.price_poll_interval);

    // Run one update immediately on startup
    try_update_price(&client, &cfg, &price_source, admin.clone(), "initial").await;

    loop {
        ticker.tick().await;
        try_update_price(&client, &cfg, &price_source, admin.clone(), "scheduled").await;
    }
}

async fn try_update_price(
    client: &RpcClient,
    cfg: &Config,
    price_source: &PriceSource,
    admin: Arc<Keypair>,
    kind: &'static str,
) {
    match price_source.fetch_price().await {
        Ok(price) => {
            if price == 0 {
                warn!("Skipped {} price update because fetched price is zero", kind);
                return;
            }
            match submit_price(client, cfg, price, admin).await {
                Ok(sig) => info!(%sig, price, "oracle price updated ({})", kind),
                Err(err) => error!(?err, "failed to submit {} price", kind),
            }
        }
        Err(err) => error!(?err, "failed to fetch {} price", kind),
    }
}

async fn submit_price(
    client: &RpcClient,
    cfg: &Config,
    new_price: u64,
    admin: Arc<Keypair>,
) -> Result<Signature> {
    use sol_usd_oracle::{accounts, instruction};

    let ix_data = instruction::UpdatePrice { new_price }.data();
    let accounts = accounts::UpdatePrice { oracle: cfg.oracle_state, admin: admin.pubkey() }
        .to_account_metas(None);

    let ix = Instruction { program_id: cfg.oracle_program_id, accounts, data: ix_data };

    let bh = client.get_latest_blockhash().await.context("fetch blockhash")?;

    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[admin.as_ref()], bh);

    let sig = client
        .send_and_confirm_transaction_with_spinner_and_commitment(
            &tx,
            CommitmentConfig::confirmed(),
        )
        .await
        .context("send price tx")?;

    Ok(sig)
}

async fn run_event_listener(cfg: Config) -> Result<()> {
    let client = PubsubClient::new(&cfg.rpc_ws).await.context("connect pubsub ws")?;

    let (mut stream, _unsub) = client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![cfg.minter_program_id.to_string()]),
            RpcTransactionLogsConfig { commitment: Some(CommitmentConfig::confirmed()) },
        )
        .await
        .context("subscribe to logs")?;

    while let Some(value) = stream.next().await {
        if let Some(parsed) = parse_token_created(&value.value, cfg.minter_program_id) {
            info!(
                target: "token_created",
                "creator={} mint={} decimals={} supply={} fee_lamports={} price={} slot={} sig={}",
                parsed.creator,
                parsed.mint,
                parsed.decimals,
                parsed.initial_supply,
                parsed.fee_lamports,
                parsed.sol_usd_price,
                parsed.slot,
                parsed.signature
            );
            if let Ok(json) = serde_json::to_string(&parsed) {
                println!("{json}");
            }
        }
    }
    Ok(())
}

fn parse_token_created(logs: &RpcLogsResponse, _program_id: Pubkey) -> Option<TokenCreatedLog> {
    let re = Regex::new(
        r"TokenCreated \{ creator: ([A-Za-z0-9]+), mint: ([A-Za-z0-9]+), decimals: (\d+), initial_supply: (\d+), fee_lamports: (\d+), sol_usd_price: (\d+), slot: (\d+) \}",
    )
    .expect("regex");

    for log in &logs.logs {
        if let Some(encoded) = log.strip_prefix("Program data: ") {
            if let Some(parsed) = parse_anchor_token_created(encoded, &logs.signature) {
                return Some(parsed);
            }
        }

        if !log.contains("TokenCreated") {
            continue;
        }

        if let Some(caps) = re.captures(log) {
            let creator = caps.get(1)?.as_str().to_string();
            let mint = caps.get(2)?.as_str().to_string();
            let decimals = caps.get(3)?.as_str().parse().ok()?;
            let initial_supply = caps.get(4)?.as_str().parse().ok()?;
            let fee_lamports = caps.get(5)?.as_str().parse().ok()?;
            let sol_usd_price = caps.get(6)?.as_str().parse().ok()?;
            let slot = caps.get(7)?.as_str().parse().ok()?;

            return Some(TokenCreatedLog {
                creator,
                mint,
                decimals,
                initial_supply,
                fee_lamports,
                sol_usd_price,
                slot,
                signature: logs.signature.clone(),
            });
        }
    }
    None
}

fn parse_anchor_token_created(encoded: &str, signature: &str) -> Option<TokenCreatedLog> {
    let decoded = BASE64_STANDARD.decode(encoded).ok()?;
    let discriminator = token_minter::TokenCreated::DISCRIMINATOR;
    let payload = decoded.strip_prefix(discriminator)?;
    let event = token_minter::TokenCreated::deserialize(&mut &payload[..]).ok()?;

    Some(TokenCreatedLog {
        creator: event.creator.to_string(),
        mint: event.mint.to_string(),
        decimals: event.decimals,
        initial_supply: event.initial_supply,
        fee_lamports: event.fee_lamports,
        sol_usd_price: event.sol_usd_price,
        slot: event.slot,
        signature: signature.to_string(),
    })
}

fn to_fixed_6(txt: &str) -> Result<u64> {
    const SCALE: u64 = 1_000_000;

    let txt = txt.trim();
    if txt.is_empty() {
        return Err(anyhow!("price is empty"));
    }

    let mut parts = txt.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(anyhow!("invalid decimal price: {txt}"));
    }

    let whole = whole.parse::<u64>().context("parse integer price part")?;
    let mut fraction_value = 0_u64;
    let mut fraction_digits = 0_u8;

    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return Err(anyhow!("invalid decimal price: {txt}"));
        }

        for digit in fraction.bytes().take(PRICE_DECIMALS as usize) {
            fraction_value = fraction_value * 10 + u64::from(digit - b'0');
            fraction_digits += 1;
        }
    }

    for _ in fraction_digits..PRICE_DECIMALS {
        fraction_value *= 10;
    }

    whole
        .checked_mul(SCALE)
        .and_then(|scaled| scaled.checked_add(fraction_value))
        .ok_or_else(|| anyhow!("fixed-6 price overflow: {txt}"))
}

#[cfg(test)]
mod tests {
    use solana_client::rpc_response::RpcLogsResponse;

    use super::*;

    fn sample_cfg(mock_price: Option<u64>, price_api_url: Option<&str>) -> Config {
        Config {
            rpc_http: "http://127.0.0.1:8899".to_string(),
            rpc_ws: "ws://127.0.0.1:8900".to_string(),
            oracle_program_id: Pubkey::new_unique(),
            oracle_state: Pubkey::new_unique(),
            minter_program_id: Pubkey::new_unique(),
            backend_keypair_path: "/tmp/id.json".to_string(),
            price_poll_interval: Duration::from_secs(60),
            mock_price,
            price_api_url: price_api_url.map(ToString::to_string),
        }
    }

    #[test]
    fn to_fixed_6_parses_integer_and_fractional_part() {
        assert_eq!(to_fixed_6("120").unwrap(), 120_000_000);
        assert_eq!(to_fixed_6("120.12").unwrap(), 120_120_000);
        assert_eq!(to_fixed_6("0.000001").unwrap(), 1);
    }

    #[test]
    fn to_fixed_6_pads_fraction_and_trims_ascii_whitespace() {
        assert_eq!(to_fixed_6(" 120.1\n").unwrap(), 120_100_000);
        assert_eq!(to_fixed_6("0002.000003").unwrap(), 2_000_003);
        assert_eq!(to_fixed_6("0.1").unwrap(), 100_000);
    }

    #[test]
    fn to_fixed_6_truncates_fraction_to_six_digits() {
        assert_eq!(to_fixed_6("1.1234569").unwrap(), 1_123_456);
    }

    #[test]
    fn to_fixed_6_rejects_invalid_input() {
        for input in ["", " ", "abc", ".1", "1.", "1..2", "-1", "+1", "1e2", "1_000", "1.2x", "x.2"]
        {
            assert!(to_fixed_6(input).is_err(), "{input:?} should be rejected");
        }
    }

    #[test]
    fn to_fixed_6_rejects_overflowing_values() {
        assert!(to_fixed_6("18446744073710.000000").is_err());
        assert!(to_fixed_6("18446744073709551616").is_err());
    }

    #[test]
    fn parse_token_created_reads_expected_fields() {
        let logs = RpcLogsResponse {
            signature: "5Yf8k3w2J3k9R8B9Q2".to_string(),
            err: None,
            logs: vec![
                "Program xyz log".to_string(),
                "Program log: TokenCreated { creator: 4N8wYzU2aB3cD4eF5gH6iJ7kL8mN9pQ1R2sT3uV4wXy, mint: 7K9mP2xQ8dW1vR6nT4cB3zY5aL7fG2hJ9sD1qW8eR4t, decimals: 6, initial_supply: 1000000, fee_lamports: 41666666, sol_usd_price: 120000000, slot: 77 }".to_string(),
            ],
        };

        let parsed = parse_token_created(&logs, Pubkey::new_unique()).expect("event should parse");
        assert_eq!(parsed.decimals, 6);
        assert_eq!(parsed.initial_supply, 1_000_000);
        assert_eq!(parsed.fee_lamports, 41_666_666);
        assert_eq!(parsed.sol_usd_price, 120_000_000);
        assert_eq!(parsed.slot, 77);
        assert_eq!(parsed.signature, logs.signature);
    }

    #[test]
    fn parse_token_created_reads_anchor_program_data_event() {
        let logs = RpcLogsResponse {
            signature: "2sKgiT6uetjUipJgEcsQUDb62qz2mM5UprBsv6vgUuh2gVSKkmfp3xgJrYT4qVvTaT92ZpZsurzXDPcreY8wZvRV".to_string(),
            err: None,
            logs: vec![
                "Program 95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc invoke [1]"
                    .to_string(),
                "Program log: Instruction: MintToken".to_string(),
                "Program data: 7BMp/4JOk6xPWqI3F1tiuFMA5fb6GNKmalUByoROQmJBVi6+BJTPhg36TZpix+WWnuyTbMaploY0dwThPn2yLXyQ5WdxRy7xBkBCDwAAAAAAash7AgAAAAAADicHAAAAAHwAAAAAAAAA"
                    .to_string(),
                "Program 95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc success"
                    .to_string(),
            ],
        };

        let parsed = parse_token_created(&logs, Pubkey::new_unique()).expect("event should parse");
        assert_eq!(parsed.decimals, 6);
        assert_eq!(parsed.initial_supply, 1_000_000);
        assert_eq!(parsed.fee_lamports, 41_666_666);
        assert_eq!(parsed.sol_usd_price, 120_000_000);
        assert_eq!(parsed.slot, 124);
        assert_eq!(parsed.signature, logs.signature);
    }

    #[test]
    fn parse_token_created_returns_none_for_unrelated_logs() {
        let logs = RpcLogsResponse {
            signature: "4m4u3z".to_string(),
            err: None,
            logs: vec!["Program log: some other event".to_string()],
        };
        assert!(parse_token_created(&logs, Pubkey::new_unique()).is_none());
    }

    #[test]
    fn parse_token_created_skips_malformed_event_lines() {
        let logs = RpcLogsResponse {
            signature: "5Yf8k3w2J3k9R8B9Q2".to_string(),
            err: None,
            logs: vec![
                "Program log: TokenCreated { creator: missing fields }".to_string(),
                "Program log: TokenCreated { creator: 4N8wYzU2aB3cD4eF5gH6iJ7kL8mN9pQ1R2sT3uV4wXy, mint: 7K9mP2xQ8dW1vR6nT4cB3zY5aL7fG2hJ9sD1qW8eR4t, decimals: 9, initial_supply: 42, fee_lamports: 100, sol_usd_price: 120000000, slot: 88 }".to_string(),
            ],
        };

        let parsed = parse_token_created(&logs, Pubkey::new_unique()).expect("event should parse");
        assert_eq!(parsed.decimals, 9);
        assert_eq!(parsed.initial_supply, 42);
        assert_eq!(parsed.slot, 88);
    }

    #[test]
    fn parse_token_created_returns_none_for_overflowing_fields() {
        let logs = RpcLogsResponse {
            signature: "5Yf8k3w2J3k9R8B9Q2".to_string(),
            err: None,
            logs: vec![
                "Program log: TokenCreated { creator: 4N8wYzU2aB3cD4eF5gH6iJ7kL8mN9pQ1R2sT3uV4wXy, mint: 7K9mP2xQ8dW1vR6nT4cB3zY5aL7fG2hJ9sD1qW8eR4t, decimals: 256, initial_supply: 1000000, fee_lamports: 41666666, sol_usd_price: 120000000, slot: 77 }".to_string(),
            ],
        };

        assert!(parse_token_created(&logs, Pubkey::new_unique()).is_none());
    }

    #[test]
    fn price_source_prefers_mock_over_url() {
        let cfg = sample_cfg(Some(123), Some("https://example.com/price"));
        let source = PriceSource::from_config(&cfg);
        match source {
            PriceSource::Mock(v) => assert_eq!(v, 123),
            PriceSource::Http { .. } => panic!("expected mock source"),
        }
    }

    #[test]
    fn price_source_uses_url_override_when_mock_absent() {
        let cfg = sample_cfg(None, Some("https://example.com/price"));
        let source = PriceSource::from_config(&cfg);
        match source {
            PriceSource::Mock(_) => panic!("expected http source"),
            PriceSource::Http { url } => assert_eq!(url, "https://example.com/price"),
        }
    }

    #[tokio::test]
    async fn price_source_fetches_mock_price_without_network() {
        assert_eq!(PriceSource::Mock(456).fetch_price().await.unwrap(), 456);
    }

    #[test]
    fn price_source_uses_default_url_when_no_override() {
        let cfg = sample_cfg(None, None);
        let source = PriceSource::from_config(&cfg);
        match source {
            PriceSource::Mock(_) => panic!("expected http source"),
            PriceSource::Http { url } => {
                assert_eq!(url, "https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT")
            }
        }
    }
}
