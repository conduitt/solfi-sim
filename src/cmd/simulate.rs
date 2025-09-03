// CHANGED: keeps the original WSOL/USDC multi-pool simulator, adds a Token-2022–aware single-market simulator,

use crate::constants::{SOLFI_MARKETS, SOLFI_PROGRAM, USDC, WSOL};
use crate::swap::{SwapDirection, create_swap_ix, create_swap_ix_generic_with_token_program}; // CHANGED
use crate::types::{AccountWithAddress, FetchMetadata};
use crate::utils::{
    token_balance,
    account_owner_program,
    token_balance_generic,
    read_token_account_mint,
    read_mint_decimals_generic,
    u64_at_offset,
};

use csv::WriterBuilder;
use eyre::eyre;
use litesvm::LiteSVM;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk::native_token::sol_to_lamports;
use solana_sdk::program_option::COption;
use solana_sdk::program_pack::Pack;
use solana_sdk::rent::Rent;
use solana_sdk::rent_collector::RENT_EXEMPT_RENT_EPOCH;
use solana_signer::Signer;
use solana_system_interface::instruction::transfer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction::sync_native;
use spl_token::state::{Account as TokenAccount, AccountState};

// ADDED: Token-2022 types
use spl_token_2022::state as token2022_state;

use std::io::stdout;

const SOLFI_PROGRAM_PATH: &str = "data/solfi.so";
const DEFAULT_SWAP_AMOUNT_SOL: f64 = 10.0;
const DEFAULT_SWAP_AMOUNT_USDC: f64 = 1000.0;
const SOL_DECIMALS: i32 = 9;
const USDC_DECIMALS: i32 = 6;

const GEN_OFFSET: usize = 464;

// ADDED: min "generated" slot across the 4 WSOL/USDC markets (for multi-pool)
fn safe_snapshot_slot_from_files() -> Option<u64> {
    let mut min_gen = u64::MAX;
    for market in crate::constants::SOLFI_MARKETS {
        if let Ok(acct) = AccountWithAddress::read_account(format!("data/account_{market}.json").into()) {
            if let Ok(gen) = u64_at_offset(acct.account.data.as_slice(), GEN_OFFSET) {
                if gen < min_gen {
                    min_gen = gen;
                }
            }
        }
    }
    if min_gen != u64::MAX { Some(min_gen) } else { None }
}

// ADDED: the saved "generated" slot for a single market (for generic sim)
fn safe_slot_for_market_from_file(market: &Pubkey) -> Option<u64> {
    if let Ok(acct) = AccountWithAddress::read_account(format!("data/account_{market}.json").into()) {
        if let Ok(gen) = u64_at_offset(acct.account.data.as_slice(), GEN_OFFSET) {
            return Some(gen);
        }
    }
    None
}

fn mk_ata_account(mint: &Pubkey, user: &Pubkey, amount: u64) -> Account {
    let ata = TokenAccount {
        mint: *mint,
        owner: *user,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0u8; TokenAccount::LEN];
    ata.pack_into_slice(&mut data);
    Account {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: RENT_EXEMPT_RENT_EPOCH,
    }
}

// ADDED: a WSOL "native" token account (is_native=Some(0)) so sync_native works inside LiteSVM
fn mk_native_wsol_account(user: &Pubkey) -> Account {
    let ata = TokenAccount {
        mint: WSOL,
        owner: *user,
        amount: 0,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::Some(0), // mark as native SOL wrapper
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0u8; TokenAccount::LEN];
    ata.pack_into_slice(&mut data);
    Account {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: RENT_EXEMPT_RENT_EPOCH,
    }
}

pub fn simulate(
    direction: SwapDirection,
    amount: Option<f64>,
    slot: Option<u64>,
    ignore_errors: bool,
    prn: bool,
) -> eyre::Result<Vec<SwapResult>> {
    let user_keypair = Keypair::new();
    let user = user_keypair.pubkey();
    let mut svm =
        LiteSVM::new().with_sysvars().with_precompiles().with_sigverify(true).with_spl_programs();

    for acct in AccountWithAddress::read_all()? {
        svm.set_account(acct.address, acct.account)?;
    }
    svm.add_program_from_file(SOLFI_PROGRAM, SOLFI_PROGRAM_PATH)?;

    let warp_slot = slot
        .or_else(safe_snapshot_slot_from_files)
        .or_else(|| FetchMetadata::read().map(|m| m.slot()));
    if let Some(s) = warp_slot {
        svm.warp_to_slot(s);
    }

    let (to_mint, from_decimals, to_decimals, in_amount_ui) = match direction {
        SwapDirection::SolToUsdc => {
            (&USDC, SOL_DECIMALS, USDC_DECIMALS, amount.unwrap_or(DEFAULT_SWAP_AMOUNT_SOL))
        }
        SwapDirection::UsdcToSol => {
            (&WSOL, USDC_DECIMALS, SOL_DECIMALS, amount.unwrap_or(DEFAULT_SWAP_AMOUNT_USDC))
        }
    };

    let amount_in_atomic = (in_amount_ui * 10f64.powi(from_decimals)) as u64;
    let total_amount_needed = amount_in_atomic * SOLFI_MARKETS.len() as u64;

    let fee_lamports = sol_to_lamports(1.0);
    match direction {
        SwapDirection::SolToUsdc => {
            let airdrop_amount = total_amount_needed + fee_lamports;
            svm.airdrop(&user, airdrop_amount)
                .map_err(|e| eyre!("failed to airdrop SOL: {}", e.err))?;
        }
        SwapDirection::UsdcToSol => {
            svm.airdrop(&user, fee_lamports)
                .map_err(|e| eyre!("failed to airdrop SOL: {}", e.err))?;
            let usdc_ata = get_associated_token_address(&user, &USDC);
            let usdc_account = mk_ata_account(&USDC, &user, total_amount_needed);
            svm.set_account(usdc_ata, usdc_account)?;
        }
    }

    // CHANGED: pre-create user ATAs in the SVM (avoid calling ATA CPI)
    let wsol_ata = get_associated_token_address(&user, &WSOL);
    let usdc_ata = get_associated_token_address(&user, &USDC);
    svm.set_account(wsol_ata, mk_native_wsol_account(&user))?;
    // if USDC ATA wasn't already seeded above (SolToUsdc path), create an empty one now
    if svm.get_account(&usdc_ata).is_none() {
        svm.set_account(usdc_ata, mk_ata_account(&USDC, &user, 0))?;
    }

    let mut wtr = WriterBuilder::new().has_headers(false).from_writer(stdout());
    let mut results = vec![];

    for market in SOLFI_MARKETS {
        let to_ata = get_associated_token_address(&user, to_mint);
        let balance_before = token_balance(&svm, &to_ata);

        let mut instructions = vec![];

        if direction == SwapDirection::SolToUsdc {
            // Wrap SOL: system transfer lamports into WSOL ATA, then sync_native
            instructions.push(transfer(&user, &wsol_ata, amount_in_atomic));
            instructions.push(sync_native(&spl_token::id(), &wsol_ata)?);
        }

        // swap (ATA-derived vaults for WSOL/USDC markets)
        instructions.push(create_swap_ix(direction, market, &user, &WSOL, &USDC, amount_in_atomic));

        let tx = Transaction::new_with_payer(&instructions, Some(&user));
        let signed_tx = Transaction::new(&[&user_keypair], tx.message, svm.latest_blockhash());

        match svm.send_transaction(signed_tx) {
            Ok(_) => {
                let balance_after = token_balance(&svm, &to_ata);
                let out_amount_atomic = balance_after.saturating_sub(balance_before);
                let out_amount_ui = out_amount_atomic as f64 / 10f64.powi(to_decimals);
                let swap_result = SwapResult {
                    market: market.to_string(),
                    in_amount: in_amount_ui,
                    out_amount: Some(out_amount_ui),
                    error: None,
                };
                if prn {
                    wtr.serialize(&swap_result)?;
                }
                results.push(swap_result);
            }
            Err(err) => {
                if !ignore_errors {
                    let swap_result = SwapResult {
                        market: market.to_string(),
                        in_amount: in_amount_ui,
                        out_amount: None,
                        error: Some(err.err.to_string()),
                    };
                    if prn {
                        wtr.serialize(&swap_result)?;
                    }
                    results.push(swap_result);
                }
            }
        }
        wtr.flush()?;
    }

    Ok(results)
}

#[derive(serde::Serialize)]
pub struct SwapResult {
    pub market: String,
    pub in_amount: f64,
    pub out_amount: Option<f64>,
    pub error: Option<String>,
}

fn to_units(amount: f64, decimals: u8) -> u64 {
    let scale = 10u128.pow(decimals as u32);
    let v = ((amount * scale as f64).round() as i128).max(0) as u128;
    u64::try_from(v).unwrap()
}

// ADDED: create a token account with the correct owner program (spl-token or spl-token-2022)
fn mk_ata_account_with_owner(
    mint: &Pubkey,
    user: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) -> Account {
    if *token_program == spl_token::id() {
        // legacy SPL Token layout
        let ata = TokenAccount {
            mint: *mint,
            owner: *user,
            amount,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        let mut data = vec![0u8; TokenAccount::LEN];
        ata.pack_into_slice(&mut data);
        Account {
            lamports: Rent::default().minimum_balance(data.len()),
            data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: RENT_EXEMPT_RENT_EPOCH,
        }
    } else {
        // Token-2022 account (no extensions for our sim)
        let ata2022 = token2022_state::Account {
            mint: *mint,
            owner: *user,
            amount,
            delegate: COption::None,
            state: token2022_state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        let mut data = vec![0u8; token2022_state::Account::LEN];
        ata2022.pack_into_slice(&mut data);
        Account {
            lamports: Rent::default().minimum_balance(data.len()),
            data,
            owner: spl_token_2022::id(),
            executable: false,
            rent_epoch: RENT_EXEMPT_RENT_EPOCH,
        }
    }
}

// ADDED: Single-market sim that supports both SPL Token and Token-2022 vaults
pub fn simulate_generic_single_market(
    market: Pubkey,
    market_vault_quote: Pubkey,
    market_vault_base: Pubkey,
    amount_ui: f64,
    direction: SwapDirection,
    slot: Option<u64>,
    prn: bool,
) -> eyre::Result<SwapResult> {
    let user_keypair = Keypair::new();
    let user = user_keypair.pubkey();

    let mut svm =
        LiteSVM::new().with_sysvars().with_precompiles().with_sigverify(true).with_spl_programs();

    for acct in AccountWithAddress::read_all()? {
        svm.set_account(acct.address, acct.account)?;
    }
    svm.add_program_from_file(SOLFI_PROGRAM, SOLFI_PROGRAM_PATH)?;

    let warp_slot = slot
        .or_else(|| safe_slot_for_market_from_file(&market))
        .or_else(|| FetchMetadata::read().map(|m| m.slot()));
    if let Some(s) = warp_slot {
        svm.warp_to_slot(s);
    }

    let quote_token_program = account_owner_program(&svm, &market_vault_quote)?;
    let base_token_program  = account_owner_program(&svm, &market_vault_base)?;
    eyre::ensure!(
        quote_token_program == base_token_program,
        "vault token programs mismatch"
    );
    let token_program_id = quote_token_program;

    let quote_mint = read_token_account_mint(&svm, &market_vault_quote)?;
    let base_mint  = read_token_account_mint(&svm, &market_vault_base)?;
    let quote_dec  = read_mint_decimals_generic(&svm, &quote_mint, &token_program_id)?;
    let base_dec   = read_mint_decimals_generic(&svm, &base_mint,  &token_program_id)?;

    let fee_lamports = sol_to_lamports(1.0);
    svm.airdrop(&user, fee_lamports)
        .map_err(|e| eyre!("failed to airdrop SOL: {}", e.err))?;

    let user_base_ata  = get_associated_token_address(&user, &base_mint);
    let user_quote_ata = get_associated_token_address(&user, &quote_mint);

    let base_zero = mk_ata_account_with_owner(&base_mint, &user, 0, &token_program_id);
    let quote_zero = mk_ata_account_with_owner(&quote_mint, &user, 0, &token_program_id);
    svm.set_account(user_base_ata, base_zero)?;
    svm.set_account(user_quote_ata, quote_zero)?;

    let (amount_in_atomic, input_is_quote) = match direction {
        SwapDirection::UsdcToSol => (to_units(amount_ui, quote_dec), true),
        SwapDirection::SolToUsdc => (to_units(amount_ui, base_dec),  false),
    };
    let seed_ata  = if input_is_quote { user_quote_ata } else { user_base_ata };
    let seed_mint = if input_is_quote { quote_mint } else { base_mint };
    let seeded = mk_ata_account_with_owner(&seed_mint, &user, amount_in_atomic, &token_program_id);
    svm.set_account(seed_ata, seeded)?;

    let (to_ata, to_decimals) = if input_is_quote {
        (user_base_ata, base_dec)
    } else {
        (user_quote_ata, quote_dec)
    };
    let balance_before = token_balance_generic(&svm, &to_ata, &token_program_id)?;

    let ix = create_swap_ix_generic_with_token_program(
        direction,
        &market,
        &user,
        &market_vault_base,
        &market_vault_quote,
        &base_mint,
        &quote_mint,
        &token_program_id,
        amount_in_atomic,
    );

    let tx = Transaction::new_with_payer(&[ix], Some(&user));
    let signed_tx = Transaction::new(&[&user_keypair], tx.message, svm.latest_blockhash());

    let mut wtr = WriterBuilder::new().has_headers(false).from_writer(stdout());
    match svm.send_transaction(signed_tx) {
        Ok(_) => {
            let balance_after = token_balance_generic(&svm, &to_ata, &token_program_id)?;
            let out_amount_atomic = balance_after.saturating_sub(balance_before);
            let out_amount_ui = out_amount_atomic as f64 / 10f64.powi(to_decimals as i32);
            let res = SwapResult {
                market: market.to_string(),
                in_amount: amount_ui,
                out_amount: Some(out_amount_ui),
                error: None,
            };
            if prn { wtr.serialize(&res)?; wtr.flush()?; }
            Ok(res)
        }
        Err(err) => {
            let res = SwapResult {
                market: market.to_string(),
                in_amount: amount_ui,
                out_amount: None,
                error: Some(err.err.to_string()),
            };
            if prn { wtr.serialize(&res)?; wtr.flush()?; }
            Ok(res)
        }
    }
}
