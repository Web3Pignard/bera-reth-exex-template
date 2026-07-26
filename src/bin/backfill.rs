//! Standalone backfill tool.
//!
//! Neither `reth import` nor swapping in the ExEx-enabled `reth` binary on an
//! already-synced node delivers historical `ExExNotification`s for blocks that were
//! processed before the ExEx was registered (the sync pipeline's `ExecutionStage` only
//! sends a `ChainCommitted` notification for blocks it actually (re-)executes; once a
//! stage's checkpoint already covers a range, `execute()` short-circuits and nothing is
//! ever recomputed for it). This tool replays that missing history directly from an
//! already-synced node's JSON-RPC endpoint: it asks `eth_getLogs` for every log matching
//! a known staking event signature over a block range, decodes them with the same logic
//! the live ExEx uses, and writes them into the indexer database.

use std::collections::HashMap;
use std::sync::Arc;

use alloy_primitives::hex;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockNumberOrTag, Filter};
use clap::Parser;
use eyre::Result;

use zg_reth_exex_template::abi;
use zg_reth_exex_template::db::Database;
use zg_reth_exex_template::event_handler::EventHandler;
use zg_reth_exex_template::event_listener::{apply_decoded_event, decode_log};
use zg_reth_exex_template::exex::{seed_genesis_validators, DEFAULT_DB_PATH};
use zg_reth_exex_template::validators::ValidatorManager;

#[derive(Parser, Debug)]
#[command(about = "Backfill the staking indexer database from an already-synced node's RPC")]
struct Args {
    /// JSON-RPC HTTP endpoint of an already-synced node (needs the `eth` namespace)
    #[arg(long)]
    rpc_url: String,

    /// First block to backfill (inclusive). Defaults to one past the indexer's last
    /// synced block (or block 1 if the database has never been synced).
    #[arg(long)]
    from_block: Option<u64>,

    /// Last block to backfill (inclusive). Defaults to the node's current tip.
    #[arg(long)]
    to_block: Option<u64>,

    /// Path to the staking indexer SQLite database file
    #[arg(long = "indexer.db-path", default_value = DEFAULT_DB_PATH)]
    db_path: String,

    /// Number of blocks to request per eth_getLogs call
    #[arg(long, default_value_t = 2000)]
    chunk_size: u64,

    /// Replay the hardcoded genesis validator seeding normally done at block #1. Only
    /// pass this when the indexer has genuinely never processed block #1 before (e.g.
    /// backfilling a brand-new database from scratch) -- passing it when block #1 was
    /// already indexed live will double-count those validators' delegated amounts.
    #[arg(long, default_value_t = false)]
    seed_genesis_validators: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let args = Args::parse();

    let db = Arc::new(Database::new(&args.db_path)?);
    let validator_manager = Arc::new(ValidatorManager::new(db.clone())?);
    let event_handler = Arc::new(EventHandler::new(db.clone(), validator_manager.clone()));

    let validator_staking_address = abi::get_validator_staking_address();

    let url: url::Url = args.rpc_url.parse()?;
    let provider = ProviderBuilder::new().connect_http(url);

    let from_block = match args.from_block {
        Some(b) => b,
        None => db.get_last_synced_block(&validator_staking_address)? + 1,
    };
    let to_block = match args.to_block {
        Some(b) => b,
        None => provider.get_block_number().await?,
    };

    if from_block > to_block {
        println!("Nothing to backfill: from_block ({from_block}) > to_block ({to_block})");
        return Ok(());
    }

    println!("Backfilling blocks {from_block}..={to_block} from {}", args.rpc_url);

    if args.seed_genesis_validators {
        if from_block > 1 {
            eyre::bail!(
                "--seed-genesis-validators requires --from-block <= 1 (got from_block={from_block})"
            );
        }
        println!("Seeding genesis validators (block #1)...");
        seed_genesis_validators(&event_handler).await?;
    }

    let topics = abi::all_event_topics();
    let mut block_timestamps: HashMap<u64, i64> = HashMap::new();

    let mut chunk_start = from_block;
    while chunk_start <= to_block {
        let chunk_end = (chunk_start + args.chunk_size - 1).min(to_block);

        let filter =
            Filter::new().from_block(chunk_start).to_block(chunk_end).event_signature(topics.clone());

        let logs = provider.get_logs(&filter).await?;
        println!("blocks {chunk_start}..={chunk_end}: {} matching logs", logs.len());

        db.begin_transaction()?;

        let chunk_result: Result<()> = async {
            for log in &logs {
                let Some(block_number) = log.block_number else { continue };
                let Some(event) = decode_log(&log.inner) else { continue };

                let block_timestamp = match log.block_timestamp {
                    Some(ts) => Some(ts as i64),
                    None => match block_timestamps.get(&block_number) {
                        Some(ts) => Some(*ts),
                        None => {
                            let block =
                                provider.get_block_by_number(BlockNumberOrTag::Number(block_number)).await?;
                            let ts = block.map(|b| b.header.timestamp as i64);
                            if let Some(ts) = ts {
                                block_timestamps.insert(block_number, ts);
                            }
                            ts
                        }
                    },
                };

                let tx_hash = log.transaction_hash.map(|h| format!("0x{}", hex::encode(h)));

                apply_decoded_event(&event_handler, event, tx_hash, block_number, block_timestamp).await?;
            }

            db.update_sync_state(&validator_staking_address, "staking_events", chunk_end)?;

            Ok(())
        }
        .await;

        match chunk_result {
            Ok(()) => db.commit_transaction()?,
            Err(err) => {
                db.rollback_transaction()?;
                return Err(err);
            }
        }

        chunk_start = chunk_end + 1;
    }

    println!("Backfill complete up to block {to_block}");
    Ok(())
}
