// Main ExEx implementation for 0gchain staking indexer
// Integrates database, event handling, and validator management into the Reth ExEx framework

use crate::db::Database;
use crate::event_handler::EventHandler;
use crate::event_listener::EventListener;
use crate::validators::ValidatorManager;
use crate::abi;
use std::{hash::Hash, sync::Arc};

use alloy_consensus::TxReceipt;
use alloy_primitives::{Address, address};
use futures::StreamExt;
use reth::api::BlockBody;
use reth::core::primitives::AlloyBlockHeader;
use reth_ethereum::primitives::SignedTransaction;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::{FullNodeComponents, FullNodeTypes, NodeTypes};
use tracing::{debug, error, info};
use itertools::izip;
/// Configuration for the staking indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub db_path: String,
    pub validator_staking_address: Address,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            db_path: "/data/0g-home/0gchaind-home/data/staking_indexer.db".to_string(),
            // db_path: "./data/.tmp/staking_indexer.db".to_string(),
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
        let event_listener = Arc::new(EventListener::new(event_handler.clone(), validator_manager.clone()));
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

                if block.number() == 1 {
                    self.event_handler.handle_validator_initialized(
                        String::from("0x57005c26d25f8a1784c6746f176ef451f65780d0f918ab9bd7800cfeb7b69165"),
                         address!("0x712A30816a8756c8FdB78dE63DB55aA70d3CF3B4"), 
                         address!("0x711b0EcB072C27DE0e50c9944d7195A51B202522"), 
                         String::from("500000000000000000000"), 
                         None, 
                         Some(0), 
                         None).await?;
            
                    self.event_handler.handle_validator_initialized(
                    String::from("0xed73214d2fe5105e5c53eb74b77775970cfb96eecc5c96999d9c81766619a3d9"),
                        address!("0x81568B27b210538869f5659035eBF506d2FC3384"), 
                        address!("0x711b0EcB072C27DE0e50c9944d7195A51B202522"), 
                        String::from("500000000000000000000"), 
                        None, 
                        Some(0), 
                        None).await?;
            
                    self.event_handler.handle_validator_initialized(
                        String::from("0x75aeacc1d243f49c9af212986dbf7277fc2728369f4c25ed17d3930bd959df26"),
                            address!("0x8EE1Af5B2791c7fa6065A02e4B579A9cC78388Fb"), 
                            address!("0x711b0EcB072C27DE0e50c9944d7195A51B202522"), 
                            String::from("500000000000000000000"), 
                            None, 
                            Some(0), 
                            None).await?;
            
                    self.event_handler.handle_validator_initialized(
                        String::from("0xf2b0f141d3fad3247c4a200138729dba2ff0bb395837b9425431b0ec39a784d1"),
                            address!("0xcb8FB96dD0F60085c9B0ef4FFAeA219caFFBF972"), 
                            address!("0x711b0EcB072C27DE0e50c9944d7195A51B202522"), 
                            String::from("500000000000000000000"), 
                            None, 
                            Some(0), 
                            None).await?;
                }
                
                for (receipt, tx) in izip!(receipts, block.body().transactions()) {
                    let tx_hash = tx.recalculate_hash();
                    self.event_listener
                        .process_logs(receipt.logs(), Some(format!("0x{}", hex::encode(&tx_hash))),  block.number(), Some(block.header().timestamp() as i64))
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