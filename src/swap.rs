// CHANGED: add a variant that accepts a dynamic token program id.

use crate::constants::SOLFI_PROGRAM;
use clap::ValueEnum;
use serde::Serialize;
use solana_pubkey::Pubkey;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::sysvar;
use spl_associated_token_account::get_associated_token_address;
use std::fmt;

const DISCRIMINATOR: u8 = 7;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, ValueEnum)]
pub enum SwapDirection {
    #[default]
    SolToUsdc,
    UsdcToSol,
}

impl fmt::Display for SwapDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapDirection::SolToUsdc => write!(f, "sol-to-usdc"),
            SwapDirection::UsdcToSol => write!(f, "usdc-to-sol"),
        }
    }
}

fn create_instruction_data(direction: SwapDirection, amount_in: u64) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(18);
    buffer.push(DISCRIMINATOR);
    buffer.extend_from_slice(&amount_in.to_le_bytes());
    buffer.resize(18, 0);
    buffer[17] = direction as u8;
    buffer
}

pub fn create_swap_ix(
    direction: SwapDirection,
    market: &Pubkey,
    user: &Pubkey,
    token_a: &Pubkey,
    token_b: &Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: SOLFI_PROGRAM,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*market, false),
            AccountMeta::new(get_associated_token_address(market, token_a), false),
            AccountMeta::new(get_associated_token_address(market, token_b), false),
            AccountMeta::new(get_associated_token_address(user, token_a), false),
            AccountMeta::new(get_associated_token_address(user, token_b), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
        ],
        data: create_instruction_data(direction, amount),
    }
}

// ADDED — explicit vaults + dynamic token program (works for Token-2022)
pub fn create_swap_ix_generic_with_token_program(
    direction: SwapDirection,
    market: &Pubkey,
    user: &Pubkey,
    market_vault_base: &Pubkey,
    market_vault_quote: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64,
) -> Instruction {
    let user_base_ata  = get_associated_token_address(user, base_mint);
    let user_quote_ata = get_associated_token_address(user, quote_mint);

    Instruction {
        program_id: SOLFI_PROGRAM,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*market, false),
            AccountMeta::new(*market_vault_base, false),
            AccountMeta::new(*market_vault_quote, false),
            AccountMeta::new(user_base_ata, false),
            AccountMeta::new(user_quote_ata, false),
            AccountMeta::new_readonly(*token_program_id, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
        ],
        data: create_instruction_data(direction, amount),
    }
}
