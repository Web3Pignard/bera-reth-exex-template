use crate::abi;
// use crate::db::Database;
use std::sync::Arc;

use alloy_eips::eip2718::Typed2718;
use alloy_primitives::Address;
use bera_reth::transaction::POL_TX_TYPE;
use futures::StreamExt;
use reth::{api::BlockBody, providers::Chain};
use reth::core::primitives::AlloyBlockHeader;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::{FullNodeComponents, FullNodeTypes, NodeTypes};
use reth_primitives_traits::SignedTransaction;
use tracing::{debug, error, info};

pub async fn my_indexer<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> eyre::Result<()> {
    while let Some(Ok(notification)) = ctx.notifications.next().await {
        // We ignore ChainReorged and ChainReverted since Berachain has fast finality via CometBFT.
        if let Some(committed) = notification.committed_chain() {
            for (block, receipts) in committed.blocks_and_receipts() {
                info!(
                    "Processing block {} with {} transactions",
                    block.number(),
                    block.body().transactions().len()
                );

                for (tx, receipt) in block.body().transactions_iter().zip(receipts.iter()) {
                    // Check if this is a PoL transaction by looking at the tx type
                    if tx.ty() == POL_TX_TYPE {
                        info!("PoL TX {}: {:?}", tx.tx_hash(), receipt);
                    }
                }
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
}

impl StakingIndexer {
    /// Creates a new staking indexer with the given configuration
    pub fn new(config: IndexerConfig) -> Self {
        info!("Initializing StakingIndexer with config: {:?}", config);

        Self {
            config,
        }
    }

    /// Handles notifications from the Reth node
    pub async fn handle_notification<Node: FullNodeComponents>(&self, notification: ExExNotification<<<Node as FullNodeTypes>::Types as NodeTypes>::Primitives>) -> eyre::Result<()> {
        // while let Some(Ok(notification)) = ctx.notifications.next().await {
        // We ignore ChainReorged and ChainReverted since Berachain has fast finality via CometBFT.
        if let Some(committed) = notification.committed_chain() {
            for (block, receipts) in committed.blocks_and_receipts() {
                
                info!(
                    "Processing block {} with {} transactions",
                    block.number(),
                    block.body().transactions().len()
                );

                for (tx, receipt) in block.body().transactions_iter().zip(receipts.iter()) {
                    // Check if this is a PoL transaction by looking at the tx type
                    if tx.ty() == POL_TX_TYPE {
                        info!("PoL TX {}: {:?}", tx.tx_hash(), receipt);
                    }
                }
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
    let indexer = Arc::new(StakingIndexer::new(config));

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