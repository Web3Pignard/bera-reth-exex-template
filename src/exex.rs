// Main ExEx implementation for 0gchain staking indexer
// Integrates database, event handling, and validator management into the Reth ExEx framework

use crate::db::Database;
use crate::event_handler::EventHandler;
use crate::event_listener::EventListener;
use crate::validators::ValidatorManager;
use crate::abi;
use std::sync::Arc;

use alloy_consensus::TxReceipt;
use alloy_primitives::Address;
use futures::StreamExt;
use reth::api::BlockBody;
use reth::core::primitives::AlloyBlockHeader;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::{FullNodeComponents, FullNodeTypes, NodeTypes};
use tracing::{debug, error, info};

pub async fn my_indexer<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> eyre::Result<()> {
    while let Some(Ok(notification)) = ctx.notifications.next().await {
        // We ignore ChainReorged and ChainReverted since 0gchain has fast finality via CometBFT.
        if let Some(committed) = notification.committed_chain() {
            for (block, receipts) in committed.blocks_and_receipts() {
                info!(
                    "Processing block {} with {} transactions",
                    block.number(),
                    block.body().transactions().len()
                );

                ctx.send_finished_height(block.num_hash())?;
            }
        }
    }

    Ok(())
}

/// Configuration for the staking indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub db_path: String,
    pub validator_staking_address: Address,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            db_path: "./staking_indexer.db".to_string(),
            validator_staking_address: abi::get_validator_staking_address(),
        }
    }
}

/// The main ExEx instance for staking indexing
pub struct StakingIndexer {
    config: IndexerConfig,
    db: Arc<Database>,
    validator_manager: Arc<ValidatorManager>,
    event_handler: Arc<EventHandler>,
    event_listener: Arc<EventListener>,
}

impl StakingIndexer {
    /// Creates a new staking indexer with the given configuration
    pub fn new(config: IndexerConfig) -> eyre::Result<Self> {
        info!("Initializing StakingIndexer with config: {:?}", config);

        // Initialize database
        let db = Arc::new(Database::new(&config.db_path)?);
        info!("Database initialized");

        // Initialize validator manager
        let validator_manager = Arc::new(ValidatorManager::new(db.clone())?);
        info!("ValidatorManager initialized"); 

        // Initialize event handler
        let event_handler = Arc::new(EventHandler::new(db.clone(), validator_manager.clone()));
        info!("EventHandler initialized");

        // Initialize event listener
        let event_listener = Arc::new(EventListener::new(event_handler.clone()));
        info!("EventListener initialized");

        Ok(Self {
            config,
            db,
            validator_manager,
            event_handler,
            event_listener,
        })
    }

    /// Handles notifications from the Reth node
    pub async fn handle_notification<Node: FullNodeComponents>(&self, notification: ExExNotification<<<Node as FullNodeTypes>::Types as NodeTypes>::Primitives>) -> eyre::Result<()> {
        // We ignore ChainReorged and ChainReverted since 0gchain has fast finality via CometBFT.
        if let Some(committed) = notification.committed_chain() {
            for (block, receipts) in committed.blocks_and_receipts() {
                info!(
                    "Processing block {} with {} transactions",
                    block.number(),
                    block.body().transactions().len()
                );
                
                for receipt in receipts {
                    self.event_listener
                        .process_logs(receipt.logs(), block.number(), Some(block.header().timestamp() as i64))
                        .await?;
                }

                // Update sync state to track progress
                self.db.update_sync_state(
                    &self.config.validator_staking_address,
                    "staking_events",
                    block.number(),
                )?;
            }
        }

        Ok(())
    }
}

/// The main ExEx function that runs alongside Reth
pub async fn staking_indexer_exex<N>(mut ctx: ExExContext<N>) -> eyre::Result<()>
where
    N: FullNodeComponents,
{
    info!("Starting Staking Indexer ExEx");

    // Initialize the indexer
    let config = IndexerConfig::default();
    let indexer = Arc::new(StakingIndexer::new(config)?);

    info!("StakingIndexer fully initialized and ready to process events");
    
    // Main loop: process notifications from Reth
    while let Some(Ok(notification)) = ctx.notifications.next().await {
        match indexer.handle_notification::<N>(notification.clone()).await {
            Ok(_) => {
                // Process logs from the notification if available
                if let Some(committed_chain) = notification.committed_chain() {
                    debug!(
                        "Processing committed chain: {} to {}",
                        committed_chain.first().number(),
                        committed_chain.tip().number()
                    );
                    
                    // Send FinishedHeight event to signal we've processed up to this block
                    ctx.events.send(ExExEvent::FinishedHeight(
                        committed_chain.tip().num_hash(),
                    ))?;
                }
            }
            Err(e) => {
                error!("Error processing notification: {}", e);
                // Continue processing despite errors
            }
        }
    }

    info!("Staking Indexer ExEx shutting down");

    Ok(())
}