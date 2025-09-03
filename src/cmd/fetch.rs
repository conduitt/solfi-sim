use crate::constants::{SOLFI_MARKETS, USDC, WSOL};
use crate::types::{AccountWithAddress, FetchMetadata};
use eyre::{eyre, Result};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use spl_associated_token_account::get_associated_token_address;
use spl_token::state as token_state;
use spl_token_2022::state as token2022_state;
use solana_sdk::program_pack::Pack;

pub async fn fetch_and_persist_accounts(rpc_url: String) -> Result<()> {
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let mut addresses: Vec<Pubkey> = vec![WSOL, USDC];
    for market in SOLFI_MARKETS {
        addresses.push(*market);
        addresses.push(get_associated_token_address(market, &WSOL));
        addresses.push(get_associated_token_address(market, &USDC));
    }

    tracing::info!("Fetching {} accounts for canonical WSOL/USDC pools…", addresses.len());

    let resp = client
        .get_multiple_accounts_with_commitment(&addresses, CommitmentConfig::processed())
        .await?;
    let results = resp
        .value
        .iter()
        .zip(addresses)
        .filter_map(|(account, address)| Some(AccountWithAddress { address, account: account.clone()? }))
        .collect::<Vec<_>>();

    for result in &results {
        result.save_to_file()?;
    }

    let slot = resp.context.slot;
    let metadata = FetchMetadata::new(slot);
    metadata.save_to_file()?;

    tracing::info!("Fetched and saved {} accounts at slot {}", results.len(), slot);
    Ok(())
}

pub async fn fetch_and_persist_single_market(
    rpc_url: String,
    market: Pubkey,
    quote_vault: Pubkey,
    base_vault: Pubkey,
) -> Result<()> {
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    tracing::info!(
        "Fetching single market {} with vaults (quote={}, base={})…",
        market, quote_vault, base_vault
    );

    let quote_acc = client
        .get_account_with_commitment(&quote_vault, CommitmentConfig::processed())
        .await?
        .value
        .ok_or_else(|| eyre!("missing quote vault account"))?;

    let base_acc = client
        .get_account_with_commitment(&base_vault, CommitmentConfig::processed())
        .await?
        .value
        .ok_or_else(|| eyre!("missing base vault account"))?;

    let quote_mint = if quote_acc.owner == spl_token::id() {
        token_state::Account::unpack(&quote_acc.data)?.mint
    } else if quote_acc.owner == spl_token_2022::id() {
        token2022_state::Account::unpack(&quote_acc.data)?.mint
    } else {
        return Err(eyre!("unsupported token program for quote vault"));
    };

    let base_mint = if base_acc.owner == spl_token::id() {
        token_state::Account::unpack(&base_acc.data)?.mint
    } else if base_acc.owner == spl_token_2022::id() {
        token2022_state::Account::unpack(&base_acc.data)?.mint
    } else {
        return Err(eyre!("unsupported token program for base vault"));
    };

    let to_fetch = vec![market, quote_vault, base_vault, quote_mint, base_mint];

    let resp = client
        .get_multiple_accounts_with_commitment(&to_fetch, CommitmentConfig::processed())
        .await?;

    let results = resp
        .value
        .iter()
        .zip(to_fetch)
        .filter_map(|(account, address)| Some(AccountWithAddress { address, account: account.clone()? }))
        .collect::<Vec<_>>();

    for result in &results {
        result.save_to_file()?;
    }

    let slot = resp.context.slot;
    let metadata = FetchMetadata::new(slot);
    metadata.save_to_file()?;

    tracing::info!(
        "Fetched and saved single market {} (+vaults+mints) at slot {} ({} accounts)",
        market,
        slot,
        results.len()
    );
    Ok(())
}
