# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Rust [Reth Execution Extension (ExEx)](https://reth.rs/exex/overview/) that runs alongside a 0gchain `0g-reth` node. It watches every committed block, extracts logs from the `ValidatorStaking` precompile/contract, and indexes validator/delegator staking activity into a local SQLite database. A separate Node.js `web-server/` provides a read-only dashboard over that database.

This repo is derived from Berachain's `bera-reth-exex-template` (see stale references below) but is retargeted at 0gchain via patched forks of `reth`, `revm`, `alloy-evm`, and `alloy` under `[patch.crates-io]` in [Cargo.toml](Cargo.toml) — always check that file for the exact pinned revs (`0gfoundation/0g-reth`, `0gfoundation/revm`, `0gfoundation/alloy-evm`, `0gfoundation/0g-alloy`) before assuming upstream reth/alloy behavior.

## Commands

### Rust ExEx (root)
- `make pr` — runs the full CI check suite locally: `cargo +nightly fmt --all -- --check`, `dprint check`, `cargo +nightly clippy --all-targets --all-features -- -D warnings`, doc build with `-D warnings`, `cargo test --locked`. Run this before considering Rust changes done.
- `make pr-fix` — auto-fixes formatting (`cargo +nightly fmt --all` + `dprint fmt`).
- `cargo build` — debug build of the `reth` binary (see `[[bin]] name = "reth"` in Cargo.toml).
- `cargo test --locked` — run all tests; `cargo test <name> --locked` for a single test.
- `cargo clippy --all-targets --all-features -- -D warnings` — lint (matches CI).
- `make docker-build` — builds the production image (`maxperf` profile, `jemalloc asm-keccak min-debug-logs` features). The resulting image is a drop-in replacement for the upstream `0g-reth`/`bera-reth` node image.
- `make start-local` — runs `scripts/start-local.sh`, which expects a sibling `../beacon-kit` checkout, builds the ExEx in debug mode, and launches BeaconKit + the ExEx node together against ports 8545/8551/3500. This script still references Berachain/BeaconKit tooling — treat it as a legacy convenience path, not the primary 0gchain deployment method.

### Web server (`web-server/`)
- `cd web-server && npm install`
- `npm start` — serves the dashboard on port 3000 by default. Env vars: `DB_PATH` (default `/data/staking_indexer.db`; note the SQLite file lives at `web-server/staking_indexer.db` in this repo for local dev), `PORT`.

## Architecture

### Data flow
`main.rs` registers `exex::staking_indexer_exex` as an installed ExEx named `"staking_indexer"` on a stock `EthereumNode`. For every `ExExNotification` with a committed chain, `StakingIndexer::handle_notification` (src/exex.rs):
1. Iterates blocks + receipts in the committed chain.
2. At block #1, seeds four hardcoded genesis validators via `handle_validator_initialized` (this is 0gchain-network-specific bootstrap data, not a general pattern to replicate).
3. For every transaction's receipt logs, calls `EventListener::process_logs`.
4. Persists sync progress per contract address via `Database::update_sync_state`.
5. Sends `ExExEvent::FinishedHeight` back to reth once a chain segment is processed.

Reorgs/reverts are intentionally ignored — the comment in `handle_notification` notes 0gchain has fast finality via CometBFT, so only `committed_chain()` is handled.

### Module responsibilities (`src/`)
- **`abi.rs`** — hardcodes the `ValidatorStaking` contract address (`VALIDATOR_STAKING_ADDRESS`) and precomputed keccak256 topic hashes (via `lazy_static`) for each event signature: `ValidatorCreated`, `ValidatorInitialized`, old/new `Delegate`, old/new `Undelegate`, `WithdrawCommission`, `WithdrawTipFee`. "Old" vs new Delegate/Undelegate exist because the on-chain event signature changed at some point (arity differs — old events lack a second indexed address); both are decoded and normalized to the same handler.
- **`event_listener.rs`** — `EventListener::process_logs` filters logs to the ValidatorStaking address or known validator addresses (via `ValidatorManager::is_validator`), dispatches by topic to a `handle_*_log` method, manually decodes ABI-encoded topics/data (no codegen — raw byte slicing of `log.topics()` / `log.data.data`), then calls the matching `EventHandler::handle_*`.
- **`event_handler.rs`** — business logic: writes `StakingEvent` rows and upserts aggregate `delegators.total_delegated` / `total_undelegated` (as decimal-string big integers, parsed as `i128`). `event_type` codes: `0=Delegate, 1=Undelegate, 2=WithdrawCommission, 3=WithdrawTipFee`.
- **`validators.rs`** — `ValidatorManager` keeps an in-memory `RwLock<Vec<Address>>` cache of known validator addresses, seeded from the DB at startup, so `event_listener` can cheaply check `is_validator` without hitting SQLite per log.
- **`db.rs`** — thread-safe (`Mutex<Connection>`) synchronous `rusqlite` wrapper. Schema is loaded once via `include_str!("../schema.sql")` and applied with `execute_batch` on every `Database::new`. All amounts are stored as `TEXT` (decimal strings) to preserve u256-scale precision — never coerce to `i64`/`f64` when touching these columns.
- **`exex.rs`** — wires the above together into `StakingIndexer` and the ExEx entrypoint. `IndexerConfig::default()` points `db_path` at `/data/0g-home/0gchaind-home/data/staking_indexer.db`, i.e. it assumes a specific containerized deployment layout, not `./data/...`.

### Database (`schema.sql`)
Four tables: `validators` (pubkey/address), `delegators` (aggregated totals), `events` (append-only log of every staking event), `sync_state` (per-contract last-synced block, used for observability/resume tracking rather than actual resume logic — the ExEx itself relies on reth's own ExEx checkpointing via `FinishedHeight`).

### Web server (`web-server/`)
Plain Express + `better-sqlite3`, opened `readonly`. Three paginated (`PAGE_SIZE=20`) JSON endpoints — `/api/validators`, `/api/delegators` (optional `with_events=1` groups that delegator's events by validator), `/api/events` — plus a static `public/index.html` dashboard. No auth; intended as an internal read-only viewer, not a public API.

## Known repo quirks worth knowing before assuming behavior
- `Cargo.toml` package name is `zg-reth-exex-template` and the binary is named `reth`, but `Dockerfile`, `Makefile` labels, and `README.md` still say `bera-reth`/Berachain in places — these are leftover from the template this repo was forked from and are not necessarily in sync with the actual 0gchain binary name used at runtime.
- `scripts/start-local.sh` and `README.md`'s Quick Start are written for the Berachain/BeaconKit local devnet flow, not 0gchain — don't treat them as the current source of truth for how to run against a real 0gchain node without verifying first.
