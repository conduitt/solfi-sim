# SolFi Simulator

Local Solana simulator for SolFi markets using [LiteSVM](https://github.com/LiteSVM/litesvm).

Runs deterministic WSOL/USDC sims across the canonical SolFi pools and—new in this fork—**single-market sims** for *any* SolFi market (e.g., FART/USDC, PENGU/USDC, ETH/USDC).

[![Forked from](https://img.shields.io/badge/forked_from-upstream-blue.svg)](https://github.com/tryghostxyz/solfi-sim)

---

## Build

```
cargo build --release
```

Optional .env:
```
RPC_URL=https://api.mainnet-beta.solana.com
```

CLI
```
./target/release/solfi-sim
Usage: solfi-sim <COMMAND>

Commands:
  fetch-accounts  Fetch pool accounts + related data (multi-pool WSOL/USDC or a single market)
  cutoffs         Print slot cutoff and other metadata from fetched pool data
  spreads         Calculate bid/ask spreads (supports --sizes and --csv)
  simulate        Simulate a single-leg swap across WSOL/USDC pools (legacy path)
  help            Print help
```
A) Multi-pool WSOL/USDC

Fetch snapshot (canonical 4 pools):
```
./target/release/solfi-sim fetch-accounts
```

Show cutoffs:
```
./target/release/solfi-sim cutoffs
```

One-leg sims:
```
./target/release/solfi-sim simulate --direction usdc-to-sol --amount 1000
./target/release/solfi-sim simulate --direction sol-to-usdc --amount 10
```

Round-trip spreads (print):
```
./target/release/solfi-sim spreads 100.0
```
CSV sweep (multi-pool)
```
./target/release/solfi-sim spreads 100 \
  --sizes 10,25,50,100,250,500,1000 \
  --csv curves_wsol_usdc.csv
```
B) Single-market (any SolFi market)

You need:

- Market pubkey (SolFi market account)
- QUOTE vault = USDC token account owned by the market
- BASE vault = asset token account owned by the market

Get them from:

- Markets: https://solscan.io/labelcloud/solfi#accounts
- Vaults: open the market → “Overview” → copy the two token accounts owned by the market; the one with USDC mint is QUOTE.

Fetch snapshot for a specific market:
```
./target/release/solfi-sim fetch-accounts \
  --market <MARKET_PUBKEY> \
  --market-token-quote <USDC_VAULT> \
  --market-token-base  <BASE_VAULT>
```

Round-trip spread (print):
```
./target/release/solfi-sim spreads 100.0 \
  --market <MARKET_PUBKEY> \
  --market-token-quote <USDC_VAULT> \
  --market-token-base  <BASE_VAULT>
```
CSV sweep (single-market)
```
./target/release/solfi-sim spreads 100 \
  --sizes 10,50,100,250,500,1000 \
  --csv curves_single_market.csv \
  --market <MARKET_PUBKEY> \
  --market-token-quote <USDC_VAULT> \
  --market-token-base  <BASE_VAULT>
```

Single-market Example
PENGU/USDC
```
# Fetch snapshot
./target/release/solfi-sim fetch-accounts \
  --market 8LbNkQgvJHkGsF6poBTRzxi3TNEFE7xHzfwQKjMWNLko \
  --market-token-quote EBgjCinutbhu2JP83vm8yP3m51zxMLgkdUZBib78XmvL \
  --market-token-base  6tjb7iHPNANWSygn7jHkjJEa9AR4wa6pwnXzNNc66Xi8

# Print
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
```
<img width="1800" height="1050" alt="image" src="https://github.com/user-attachments/assets/0e748602-a16b-49da-a21e-e37ed5a3d880" />
