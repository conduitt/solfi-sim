use crate::swap::SwapDirection;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Command {
    FetchAccounts {
        #[arg(long)]
        market: Option<String>,
        #[arg(long = "market-token-quote")]
        market_token_quote: Option<String>,
        #[arg(long = "market-token-base")]
        market_token_base: Option<String>,
    },

    Cutoffs,

    Spreads {
        starting_usdc: f64,
        #[arg(long, value_delimiter = ',', value_parser = clap::value_parser!(f64))]
        sizes: Option<Vec<f64>>,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        market: Option<String>,
        #[arg(long = "market-token-quote")]
        market_token_quote: Option<String>,
        #[arg(long = "market-token-base")]
        market_token_base: Option<String>,
        #[arg(long)]
        slot: Option<u64>,
    },

    Simulate {
        #[arg(short, long)]
        amount: Option<f64>,
        #[arg(short, long, default_value_t = SwapDirection::SolToUsdc)]
        direction: SwapDirection,
        #[arg(short, long)]
        slot: Option<u64>,
        #[arg(long)]
        ignore_errors: bool,
    },
}

#[derive(Debug, Parser)]
#[clap(name = "app", version)]
pub struct App {
    #[clap(subcommand)]
    pub command: Command,
}
