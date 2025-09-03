use crate::cmd::{display_cutoffs, simulate_all, simulate_generic_single_market};
use crate::swap::SwapDirection;
use crate::types::AccountWithAddress;
use crate::utils::u64_at_offset;
use csv::WriterBuilder;
use eyre::WrapErr;
use solana_pubkey::Pubkey;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

const GEN_OFFSET: usize = 464;

#[derive(serde::Serialize, Debug, Clone)]
struct SpreadRow {
    amount_usdc: f64,
    market: String,
    buy_price: f64,
    sell_price: f64,
    spread_usd: f64,
    spread_bps: f64,
}

pub fn run_spreads(
    starting_usdc: f64,
    sizes: Option<Vec<f64>>,
    csv: Option<&Path>,
    market: Option<&str>,
    market_token_quote: Option<&str>,
    market_token_base: Option<&str>,
    slot_opt: Option<u64>,
) -> eyre::Result<()> {
    let sweep = sizes.unwrap_or_else(|| vec![starting_usdc]);

    if let (Some(mkt), Some(quote), Some(base)) = (market, market_token_quote, market_token_base) {
        if let Some(csv_path) = csv {
            let mut w = WriterBuilder::new().has_headers(true).from_path(csv_path)?;
            for amt in &sweep {
                if let Some(row) = compute_single_market_row(*amt, mkt, quote, base, slot_opt)? {
                    w.serialize(&row)?;
                }
            }
            w.flush()?;
        } else {
            if let Some(gen) = read_generated_slot_for_market(mkt)? {
                println!("== using market snapshot generated slot {gen} ==\n");
            }
            let multi = sweep.len() > 1;
            for amt in &sweep {
                print_single_market_one_size(*amt, mkt, quote, base, slot_opt)?;
                if multi {
                    println!();
                }
            }
        }
        return Ok(());
    }

    if let Some(csv_path) = csv {
        let mut w = WriterBuilder::new().has_headers(true).from_path(csv_path)?;
        for amt in &sweep {
            let rows = compute_multi_pool_rows(*amt)?;
            for r in rows {
                w.serialize(&r)?;
            }
        }
        w.flush()?;
    } else {
        display_cutoffs();
        for (i, amt) in sweep.iter().enumerate() {
            if i == 0 {
                println!("\nCalculating spreads based on a round trip starting with {:.2} USDC...\n", amt);
            } else {
                println!("\n== Amount: {:.2} USDC ==\n", amt);
            }
            print_multi_pool_one_size(*amt)?;
        }
    }

    Ok(())
}

fn print_single_market_one_size(
    usdc_amount_in: f64,
    market: &str,
    quote_vault: &str,
    base_vault: &str,
    slot_opt: Option<u64>,
) -> eyre::Result<()> {
    let Some(row) = compute_single_market_row(usdc_amount_in, market, quote_vault, base_vault, slot_opt)? else {
        return Ok(());
    };

    println!("Calculating single-market spread (round trip) with {:.2} USDC on {}...\n",
             row.amount_usdc, market);
    println!("--- Market: {} ---", row.market);
    println!("  Buy BASE at:  ${:<10.6} (Ask)", row.buy_price);
    println!("  Sell BASE at: ${:<10.6} (Bid)",  row.sell_price);
    println!("  Spread:       ${:<10.6}",        row.spread_usd);
    println!("  Spread:       {:<10.2} bps",     row.spread_bps);
    Ok(())
}

fn compute_single_market_row(
    usdc_amount_in: f64,
    market: &str,
    quote_vault: &str,
    base_vault: &str,
    slot_opt: Option<u64>,
) -> eyre::Result<Option<SpreadRow>> {
    let market_pk = Pubkey::from_str(market)?;
    let quote_vault_pk = Pubkey::from_str(quote_vault)?;
    let base_vault_pk  = Pubkey::from_str(base_vault)?;

    let buy = simulate_generic_single_market(
        market_pk,
        quote_vault_pk,
        base_vault_pk,
        usdc_amount_in,
        SwapDirection::UsdcToSol,
        slot_opt,
        false,
    )?;
    let Some(base_out) = buy.out_amount else {
        println!("Buy leg failed: {:?}", buy.error);
        return Ok(None);
    };

    let sell = simulate_generic_single_market(
        market_pk,
        quote_vault_pk,
        base_vault_pk,
        base_out,
        SwapDirection::SolToUsdc,
        slot_opt,
        false,
    )?;
    let Some(usdc_out_final) = sell.out_amount else {
        println!("Sell leg failed: {:?}", sell.error);
        return Ok(None);
    };

    let buy_price  = usdc_amount_in / base_out;
    let sell_price = usdc_out_final / base_out;
    if !buy_price.is_finite() || !sell_price.is_finite() || buy_price <= 0.0 || sell_price <= 0.0 {
        return Ok(None);
    }

    let spread_usdc = buy_price - sell_price;
    let mid = (buy_price + sell_price) / 2.0;
    if mid <= 0.0 { return Ok(None); }
    let spread_bps = (spread_usdc / mid) * 10_000.0;

    Ok(Some(SpreadRow {
        amount_usdc: usdc_amount_in,
        market: market.to_string(),
        buy_price,
        sell_price,
        spread_usd: spread_usdc,
        spread_bps,
    }))
}

fn print_multi_pool_one_size(usdc_amount_in: f64) -> eyre::Result<()> {
    let mut rows = compute_multi_pool_rows(usdc_amount_in)?;
    if rows.is_empty() {
        println!("Could not complete a round-trip simulation on any market.");
        return Ok(());
    }
    rows.sort_by(|a, b| a.spread_bps.partial_cmp(&b.spread_bps).unwrap());
    for r in rows {
        println!("--- Market: {} ---", r.market);
        println!("  Buy SOL at:  ${:<10.4} (Ask)", r.buy_price);
        println!("  Sell SOL at: ${:<10.4} (Bid)", r.sell_price);
        println!("  Spread:      ${:<10.6}",       r.spread_usd);
        println!("  Spread:      {:<10.2} bps\n",  r.spread_bps);
    }
    Ok(())
}

fn compute_multi_pool_rows(usdc_amount_in: f64) -> eyre::Result<Vec<SpreadRow>> {
    let buy_side_results =
        simulate_all(SwapDirection::UsdcToSol, Some(usdc_amount_in), None, true, false)?;

    let sol_outputs_by_market: HashMap<String, f64> = buy_side_results
        .into_iter()
        .filter_map(|r| r.out_amount.map(|sol_out| (r.market, sol_out)))
        .collect();

    if sol_outputs_by_market.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows = Vec::new();

    for (market, sol_out) in sol_outputs_by_market {
        if sol_out <= 0.0 {
            continue;
        }

        if let Ok(sell_results) =
            simulate_all(SwapDirection::SolToUsdc, Some(sol_out), None, true, false)
        {
            if let Some(sell_result) = sell_results.into_iter().find(|r| r.market == market) {
                if let Some(usdc_out_final) = sell_result.out_amount {
                    let buy_price = usdc_amount_in / sol_out;
                    let sell_price = usdc_out_final / sol_out;

                    if buy_price > 0.0 && sell_price > 0.0 {
                        let spread_usdc = buy_price - sell_price;
                        let mid_price = (buy_price + sell_price) / 2.0;
                        if mid_price <= 0.0 {
                            continue;
                        }
                        let spread_bps = (spread_usdc / mid_price) * 10_000.0;

                        rows.push(SpreadRow {
                            amount_usdc: usdc_amount_in,
                            market: market.clone(),
                            buy_price,
                            sell_price,
                            spread_usd: spread_usdc,
                            spread_bps,
                        });
                    }
                }
            }
        }
    }

    Ok(rows)
}

fn read_generated_slot_for_market(market: &str) -> eyre::Result<Option<u64>> {
    let market_pk = Pubkey::from_str(market)
        .wrap_err_with(|| format!("invalid market pubkey: {market}"))?;
    match AccountWithAddress::read_account(format!("data/account_{market_pk}.json").into()) {
        Ok(acct) => {
            let gen = u64_at_offset(acct.account.data.as_slice(), GEN_OFFSET)
                .wrap_err("failed to read generated slot")?;
            Ok(Some(gen))
        }
        Err(_) => Ok(None),
    }
}
