// CHANGED: adds owner detection and Token-2022 compatible helpers.

use litesvm::LiteSVM;
use solana_pubkey::Pubkey;
use solana_sdk::program_pack::Pack;
use spl_token::state::{Account as AccountState, Mint};

// ADDED: Token-2022 imports
use spl_token_2022::{self, state as token2022_state};

pub fn token_balance(svm: &LiteSVM, pubkey: &Pubkey) -> u64 {
    let account = svm.get_account(pubkey).unwrap_or_default();
    let state = AccountState::unpack(&account.data).ok().unwrap_or_default();
    state.amount
}

pub fn u64_at_offset(data: &[u8], offset: usize) -> eyre::Result<u64> {
    let bytes = &data[offset..offset + 8];
    Ok(u64::from_le_bytes(bytes.try_into()?))
}

pub fn account_owner_program(svm: &LiteSVM, pubkey: &Pubkey) -> eyre::Result<Pubkey> {
    let acc = svm.get_account(pubkey).ok_or_else(|| eyre::eyre!("missing account"))?;
    Ok(acc.owner)
}

pub fn token_balance_generic(svm: &LiteSVM, pubkey: &Pubkey, token_program: &Pubkey) -> eyre::Result<u64> {
    let acc = svm.get_account(pubkey).ok_or_else(|| eyre::eyre!("missing token account"))?;
    if *token_program == spl_token::id() {
        let state = AccountState::unpack(&acc.data)?;
        Ok(state.amount)
    } else if *token_program == spl_token_2022::id() {
        let state = token2022_state::Account::unpack(&acc.data)?;
        Ok(state.amount)
    } else {
        eyre::bail!("unsupported token program: {token_program}")
    }
}

pub fn read_token_account_mint(svm: &LiteSVM, token_account: &Pubkey) -> eyre::Result<Pubkey> {
    let acc = svm.get_account(token_account).ok_or_else(|| eyre::eyre!("missing token account"))?;
    if let Ok(ta) = AccountState::unpack(&acc.data) {
        return Ok(ta.mint);
    }
    if let Ok(ta) = token2022_state::Account::unpack(&acc.data) {
        return Ok(ta.mint);
    }
    eyre::bail!("could not decode token account mint for {token_account}")
}

pub fn read_mint_decimals_generic(svm: &LiteSVM, mint: &Pubkey, token_program: &Pubkey) -> eyre::Result<u8> {
    let acc = svm.get_account(mint).ok_or_else(|| eyre::eyre!("missing mint account"))?;
    if *token_program == spl_token::id() {
        let m = Mint::unpack(&acc.data)?;
        Ok(m.decimals)
    } else if *token_program == spl_token_2022::id() {
        let m = token2022_state::Mint::unpack(&acc.data)?;
        Ok(m.decimals)
    } else {
        eyre::bail!("unsupported token program for mint decimals: {token_program}")
    }
}
