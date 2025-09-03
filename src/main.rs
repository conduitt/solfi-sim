mod args;
mod cmd;
mod constants;
mod swap;
mod types;
mod utils;

use crate::args::{App, Command};
use crate::cmd::{
    fetch_and_persist_accounts,
    fetch_and_persist_single_market,
    display_cutoffs,
    simulate_all as simulate,
    run_spreads,
};
use crate::constants::DEFAULT_RPC_URL;
use clap::Parser;
use dotenv::dotenv;
use solana_pubkey::Pubkey;
use std::str::FromStr;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cmd = App::parse().command;

    match cmd {
        Command::FetchAccounts { market, market_token_quote, market_token_base } => {
            let rpc_url = {
                let _ = dotenv().ok();
                std::env::var("RPC_URL").ok().filter(|url| !url.trim().is_empty()).unwrap_or_else(
                    || {
                        tracing::warn!("No RPC_URL found in env. Using {}", DEFAULT_RPC_URL);
                        DEFAULT_RPC_URL.to_string()
                    },
                )
            };

            match (market, market_token_quote, market_token_base) {
                (Some(m), Some(q), Some(b)) => {
                    let m = Pubkey::from_str(&m)?;
                    let q = Pubkey::from_str(&q)?;
                    let b = Pubkey::from_str(&b)?;
                    fetch_and_persist_single_market(rpc_url, m, q, b).await?
                }
                (None, None, None) => {
                    fetch_and_persist_accounts(rpc_url).await?
                }
                _ => {
                    eyre::bail!("When using --market mode, you must provide all of: --market, --market-token-quote, --market-token-base");
                }
            }
        }
        Command::Cutoffs => display_cutoffs(),
        Command::Spreads {
            starting_usdc,
            sizes,
            csv,
            market,
            market_token_quote,
            market_token_base,
            slot,
        } => {
            let csv_path = csv.as_ref().map(|p| p.as_path());
            run_spreads(
                starting_usdc,
                sizes,
                csv_path,
                market.as_deref(),
                market_token_quote.as_deref(),
                market_token_base.as_deref(),
                slot,
            )?;
        }
        Command::Simulate { amount, direction, slot, ignore_errors } => {
            simulate(direction, amount, slot, ignore_errors, true)?;
        }
    }

    Ok(())
}
