# SolFi Simulator

Local Solana simulator for SolFi markets using [LiteSVM](https://github.com/LiteSVM/litesvm).

Runs **deterministic** WSOL/USDC sims across the canonical SolFi pools and—new in this fork—**single-market sims** for *any* SolFi market (e.g., FART/USDC, PENGU/USDC, ETH/USDC), including **Token-2022** vaults.

[![Forked from](https://img.shields.io/badge/forked_from-upstream-blue.svg)](https://github.com/tryghostxyz/solfi-sim)

---

## Origin & Attribution

Forked from the original WSOL/USDC simulator by **tryghostxyz**:  
Upstream: https://github.com/tryghostxyz/solfi-sim

We extended it to support **arbitrary SolFi markets**, **Token-2022**, **auto safe-slot**, and **CSV sweeps**, while keeping the original flow.

---

## What’s in this fork

- **Single-market simulator (any SolFi market)** — pass market pubkey + its two vaults (QUOTE=USDC, BASE=asset).
- **Token-2022 support** — auto-detects token program from vault owner.
- **Auto safe-slot warp** — avoids cutoff rejections (multi-pool: min generated across 4 pools; single-market: that market’s generated slot).
- **No ATA CPI in SVM** — user ATAs created directly; native WSOL + `sync_native`.
- **Generic swap IX** — explicit vaults/mints + dynamic token program id.
- **CSV output + size sweeps** — `spreads --sizes ... --csv file.csv` for curve plotting.

---

## Build

```bash
cargo build --release
Optional .env:

env
Copy code
RPC_URL=https://api.mainnet-beta.solana.com
CLI
sql
Copy code
./target/release/solfi-sim
Usage: solfi-sim <COMMAND>

Commands:
  fetch-accounts  Fetch pool accounts + related data (multi-pool WSOL/USDC or a single market)
  cutoffs         Print slot cutoff and other metadata from fetched pool data
  spreads         Calculate bid/ask spreads (supports --sizes and --csv)
  simulate        Simulate a single-leg swap across WSOL/USDC pools (legacy path)
  help            Print help
A) Multi-pool WSOL/USDC
Fetch snapshot (canonical 4 pools):

bash
Copy code
./target/release/solfi-sim fetch-accounts
Show cutoffs:

bash
Copy code
./target/release/solfi-sim cutoffs
One-leg sims:

bash
Copy code
./target/release/solfi-sim simulate --direction usdc-to-sol --amount 1000
./target/release/solfi-sim simulate --direction sol-to-usdc --amount 10
Round-trip spreads (pretty print):

bash
Copy code
./target/release/solfi-sim spreads 100.0
CSV sweep (multi-pool)
bash
Copy code
./target/release/solfi-sim spreads 100 \
  --sizes 10,25,50,100,250,500,1000 \
  --csv curves_wsol_usdc.csv
B) Single-market (any SolFi market)
You need:

Market pubkey (SolFi market account)

QUOTE vault = USDC token account owned by the market

BASE vault = asset token account owned by the market

Tips:

Markets: https://solscan.io/labelcloud/solfi#accounts

Vaults: open the market → “Overview” → copy the two token accounts owned by the market; the one with USDC mint is QUOTE.

Fetch snapshot for a specific market:

bash
Copy code
./target/release/solfi-sim fetch-accounts \
  --market <MARKET_PUBKEY> \
  --market-token-quote <USDC_VAULT> \
  --market-token-base  <BASE_VAULT>
Round-trip spread (pretty print):

bash
Copy code
./target/release/solfi-sim spreads 100.0 \
  --market <MARKET_PUBKEY> \
  --market-token-quote <USDC_VAULT> \
  --market-token-base  <BASE_VAULT>
CSV sweep (single-market)
bash
Copy code
./target/release/solfi-sim spreads 100 \
  --sizes 10,50,100,250,500,1000 \
  --csv curves_single_market.csv \
  --market <MARKET_PUBKEY> \
  --market-token-quote <USDC_VAULT> \
  --market-token-base  <BASE_VAULT>
CSV schema

Copy code
amount_usdc,market,buy_price,sell_price,spread_usd,spread_bps
Single-market Examples
FART/USDC
bash
Copy code
# Fetch snapshot
./target/release/solfi-sim fetch-accounts \
  --market FeLULW5RCKDT2NuThu4vrAQ8BjH5YbU2Y7GbiZXYohm8 \
  --market-token-quote 81zFSjrz9tDu9ixkKToGDGhiX7Q18T8jG6DiwZVXrt3u \
  --market-token-base  8tpF5NrzaWYU1RjbtjuToMWMRebgqGfiMRYRf46ih9Rc

# Pretty print
./target/release/solfi-sim spreads 1000.0 \
  --market FeLULW5RCKDT2NuThu4vrAQ8BjH5YbU2Y7GbiZXYohm8 \
  --market-token-quote 81zFSjrz9tDu9ixkKToGDGhiX7Q18T8jG6DiwZVXrt3u \
  --market-token-base  8tpF5NrzaWYU1RjbtjuToMWMRebgqGfiMRYRf46ih9Rc

# CSV sweep
./target/release/solfi-sim spreads 100 \
  --sizes 10,50,100,250,500,1000 \
  --csv curves_fart_usdc.csv \
  --market FeLULW5RCKDT2NuThu4vrAQ8BjH5YbU2Y7GbiZXYohm8 \
  --market-token-quote 81zFSjrz9tDu9ixkKToGDGhiX7Q18T8jG6DiwZVXrt3u \
  --market-token-base  8tpF5NrzaWYU1RjbtjuToMWMRebgqGfiMRYRf46ih9Rc
PENGU/USDC
bash
Copy code
# Fetch snapshot
./target/release/solfi-sim fetch-accounts \
  --market 8LbNkQgvJHkGsF6poBTRzxi3TNEFE7xHzfwQKjMWNLko \
  --market-token-quote EBgjCinutbhu2JP83vm8yP3m51zxMLgkdUZBib78XmvL \
  --market-token-base  6tjb7iHPNANWSygn7jHkjJEa9AR4wa6pwnXzNNc66Xi8

# Pretty print
./target/release/solfi-sim spreads 100.0 \
  --market 8LbNkQgvJHkGsF6poBTRzxi3TNEFE7xHzfwQKjMWNLko \
  --market-token-quote EBgjCinutbhu2JP83vm8yP3m51zxMLgkdUZBib78XmvL \
  --market-token-base  6tjb7iHPNANWSygn7jHkjJEa9AR4wa6pwnXzNNc66Xi8

# CSV sweep
./target/release/solfi-sim spreads 100 \
  --sizes 10,50,100,250,500,1000 \
  --csv curves_pengu_usdc.csv \
  --market 8LbNkQgvJHkGsF6poBTRzxi3TNEFE7xHzfwQKjMWNLko \
  --market-token-quote EBgjCinutbhu2JP83vm8yP3m51zxMLgkdUZBib78XmvL \
  --market-token-base  6tjb7iHPNANWSygn7jHkjJEa9AR4wa6pwnXzNNc66Xi8
Slot handling
Multi-pool: auto-warps to the minimum generated slot across the WSOL/USDC markets.

Single-market: auto-warps to that market’s saved generated slot.

No need to pass --slot in normal use.

Troubleshooting (quick)
custom program error 0x4 on buy leg → vaults likely swapped; QUOTE must be USDC.

“Invalid account owner” / ATA issues → update to the latest build (we pre-create ATAs and wrap SOL via sync_native).

0x10 / 0x12 / 0x17 → sim past cutoff; refetch and rely on auto safe-slot.

How it works (brief)
Executes SolFi program (data/solfi.so) in LiteSVM against a saved snapshot.

Detects SPL vs Token-2022 from the owner of the vault accounts.

Creates user token accounts locally; supports native WSOL + sync_native.

Reads generated slot(s) at offset 464 from the market account and warps SVM.
