// Event listener module for monitoring and processing blockchain events
// Extracts events from blocks and routes them to appropriate handlers

use crate::abi;
use crate::event_handler::EventHandler;
use alloy_primitives::{Address, U256};
use eyre::Result;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Represents an event listener that processes blocks
pub struct EventListener {
    event_handler: Arc<EventHandler>,
}

impl EventListener {
    /// Creates a new event listener
    pub fn new(event_handler: Arc<EventHandler>) -> Self {
        Self { event_handler }
    }

    /// Processes a block and extracts relevant events
    pub async fn process_block(&self, _block: &reth_primitives::Block) -> Result<()> {
        // Note: In ExEx context, logs are typically provided through notifications
        // This method is a placeholder for the general structure
        Ok(())
    }

    /// Processes a transaction receipt's logs
    pub async fn process_logs(
        &self,
        logs: &[reth_primitives::Log],
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        for log in logs {
            debug!(
                "Processing log from address: {}, topics: {}",
                log.address,
                log.topics().len()
            );

            // Check if this log is from the ValidatorStaking contract
            if log.address != abi::get_validator_staking_address() {
                continue;
            }

            // Get the event type from the first topic
            if log.topics().is_empty() {
                warn!("Log has no topics");
                continue;
            }

            let event_topic = *log.topics().first().unwrap();

            // Route to appropriate handler based on event type
            if abi::is_validator_created_event(&event_topic.into()) {
                self.handle_validator_created_log(log, block_number, block_timestamp)
                    .await?;
            } else if abi::is_delegate_event(&event_topic.into()) {
                self.handle_delegate_log(log, block_number, block_timestamp)
                    .await?;
            } else if abi::is_undelegate_event(&event_topic.into()) {
                self.handle_undelegate_log(log, block_number, block_timestamp)
                    .await?;
            } else if abi::is_redelegate_event(&event_topic.into()) {
                self.handle_redelegate_log(log, block_number, block_timestamp)
                    .await?;
            }
        }

        Ok(())
    }

    /// Handles ValidatorCreated event log
    async fn handle_validator_created_log(
        &self,
        log: &reth_primitives::Log,
        _block_number: u64,
        _block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing ValidatorCreated event");

        // ValidatorCreated(bytes indexed pubkey, address indexed validator)
        // Topics: [event_signature, indexed_pubkey, indexed_validator]
        // Data: (empty for indexed parameters)

        if log.topics().len() < 3 {
            warn!("ValidatorCreated log has insufficient topics");
            return Ok(());
        }

        // Extract indexed parameters from topics
        let pubkey_hash = *log.topics().get(1).unwrap();
        let validator_address = Address::from(*log.topics().get(2).unwrap());

        // Note: In this case, pubkey is hashed in the topic
        // In actual implementation, we need to get the original pubkey from the transaction data
        // or keep a separate mapping

        self.event_handler
            .handle_validator_created(pubkey_hash.to_vec(), validator_address)
            .await?;

        Ok(())
    }

    /// Handles Delegate event log
    async fn handle_delegate_log(
        &self,
        log: &reth_primitives::Log,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing Delegate event");

        // Delegate(address indexed payerAddress, address indexed delegatorAddress, uint amount, uint shares)
        // Topics: [event_signature, indexed_payer, indexed_delegator]
        // Data: [amount, shares]

        if log.topics().len() < 3 {
            warn!("Delegate log has insufficient topics");
            return Ok(());
        }

        let payer_address = Address::from(*log.topics().get(1).unwrap());
        let delegator_address = Address::from(*log.topics().get(2).unwrap());
        let validator_address = log.address;

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.as_ref();
        if data.len() < 64 {
            warn!("Delegate log has insufficient data");
            return Ok(());
        }

        let amount = U256::from_be_bytes(data[0..32].try_into()?);
        let shares = U256::from_be_bytes(data[32..64].try_into()?);

        self.event_handler
            .handle_delegate(
                payer_address,
                delegator_address,
                validator_address,
                amount.to_string(),
                shares.to_string(),
                None,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles Undelegate event log
    async fn handle_undelegate_log(
        &self,
        log: &reth_primitives::Log,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing Undelegate event");

        // Undelegate(address indexed delegatorAddress, address indexed withdrawalAddress, uint shares, uint amount)
        // Topics: [event_signature, indexed_delegator, indexed_withdrawal]
        // Data: [shares, amount]

        if log.topics().len() < 3 {
            warn!("Undelegate log has insufficient topics");
            return Ok(());
        }

        let delegator_address = Address::from(*log.topics().get(1).unwrap());
        let withdrawal_address = Address::from(*log.topics().get(2).unwrap());
        let validator_address = log.address;

        // Decode data: shares and amount (each 32 bytes)
        let data = log.data.as_ref();
        if data.len() < 64 {
            warn!("Undelegate log has insufficient data");
            return Ok(());
        }

        let shares = U256::from_be_bytes(data[0..32].try_into()?);
        let amount = U256::from_be_bytes(data[32..64].try_into()?);

        self.event_handler
            .handle_undelegate(
                delegator_address,
                withdrawal_address,
                validator_address,
                shares.to_string(),
                amount.to_string(),
                None,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles Redelegate event log
    async fn handle_redelegate_log(
        &self,
        log: &reth_primitives::Log,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing Redelegate event");

        // Redelegate(address indexed ownerAddress, address indexed delegatorAddress, uint amount, uint shares)
        // Topics: [event_signature, indexed_owner, indexed_delegator]
        // Data: [amount, shares]

        if log.topics().len() < 3 {
            warn!("Redelegate log has insufficient topics");
            return Ok(());
        }

        let owner_address = Address::from(*log.topics().get(1).unwrap());
        let validator_address = log.address;

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.as_ref();
        if data.len() < 64 {
            warn!("Redelegate log has insufficient data");
            return Ok(());
        }

        let amount = U256::from_be_bytes(data[0..32].try_into()?);
        let shares = U256::from_be_bytes(data[32..64].try_into()?);

        self.event_handler
            .handle_redelegate(
                owner_address,
                validator_address,
                amount.to_string(),
                shares.to_string(),
                None,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }
}