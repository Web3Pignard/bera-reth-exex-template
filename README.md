# Bera-Reth ExEx Template

Template for building indexers, trading bots, real-time analytics and more on Berachain using [Execution Extensions (ExEx)](https://reth.rs/exex/overview/).


## Quick Start

```bash
# 1. Clone this template (or your version) and beacon-kit
git clone https://github.com/berachain/bera-reth-exex-template.git
cd bera-reth-exex-template
git clone https://github.com/berachain/beacon-kit.git ../beacon-kit

# 2. Run with BeaconKit integration
make start-local  # Runs indefinitely, Ctrl+C to stop
```

This starts both BeaconKit and Bera-Reth with the ExEx, logging PoL transactions in real-time.

## Production Deployment

For mainnet and testnet deployments, build the production Docker image:

```bash
make docker-build
```

The resulting image can be used as a **drop-in replacement** for the official bera-reth Docker image, as long as the bera-reth version in `Cargo.toml` matches the required version for your network. Simply replace `berachain/bera-reth` with your custom ExEx image in your deployment configuration.

## Using This Template

When using this repository as a template, update the following:

### Required Changes
1. **Package name** in `Cargo.toml`:
   ```toml
   [package]
   name = "your-exex-name"  # Change from "bera-reth-exex-template"
   ```

2. **Docker labels** in `Dockerfile`:
   ```dockerfile
   LABEL org.opencontainers.image.source=https://github.com/your-org/your-repo
   ```


## Backfilling Historical Events

The ExEx only sees blocks that are committed *after* it starts running. If you:

- ran `reth import` to bulk-import blocks before switching to this ExEx-enabled binary, or
- already had a plain (non-ExEx) `reth` node synced and then swapped in this binary,

then none of that pre-existing history was ever delivered to the ExEx (the sync
pipeline's execution stage only emits a notification for blocks it actually
(re-)executes, and short-circuits once its checkpoint already covers the target
range) — those blocks' staking events are simply missing from the database.

The `backfill` binary fills that gap by querying an already-synced node's own
JSON-RPC for every log matching a known staking event signature and replaying
them through the same decoding/handling logic the live ExEx uses:

```bash
# Build both binaries (same FEATURES/PROFILE as `make build`)
make build

# Backfill from block 1 up through <H>, where <H> is the chain tip you noted
# *before* switching to this ExEx binary -- pass it explicitly rather than
# relying on the default (current tip), so the range doesn't overlap with
# blocks the live ExEx has already indexed.
./target/release/backfill \
  --rpc-url http://localhost:8545 \
  --indexer.db-path /data/0g-home/0gchaind-home/data/staking_indexer.db \
  --to-block <H>
```

Useful flags:

- `--from-block` / `--to-block` — inclusive block range. `--from-block` defaults to
  one past `sync_state.last_synced_block` (or block 1 if the database has never
  been synced), so re-running the tool with no arguments resumes where it left off.
- `--chunk-size` — blocks per `eth_getLogs` call (default `2000`).
- `--seed-genesis-validators` — replays the hardcoded block-#1 genesis validator
  seeding. Only pass this when backfilling a brand-new database from genesis
  (`--from-block <= 1`) that has never had block #1 indexed live; passing it
  otherwise double-counts those validators' delegated amounts.

Run `./target/release/backfill --help` for the full list.

## Development

Edit `src/exex.rs` to customize the exex. Find more information on the [official docs](https://reth.rs/exex/overview/#how-do-i-build-an-execution-extension)


Run `make help` to see all available commands.

## License

Apache-2.0