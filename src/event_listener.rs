// Event listener module for monitoring and processing blockchain events
// Extracts events from blocks and routes them to appropriate handlers

use crate::abi;
use crate::event_handler::EventHandler;
use crate::validators::ValidatorManager;
use alloy_primitives::{Address, U256};
use eyre::Result;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use alloy_primitives::Log; 

/// Represents an event listener that processes blocks
pub struct EventListener {
    event_handler: Arc<EventHandler>,
    validator_manager: Arc<ValidatorManager>,
}

impl EventListener {
    /// Creates a new event listener
    pub fn new(event_handler: Arc<EventHandler>, validator_manager: Arc<ValidatorManager>) -> Self {
        Self { event_handler, validator_manager }
    }

    /// Processes a block and extracts relevant events
    // pub async fn process_block(&self, _block: &Block) -> Result<()> {
    //     // Note: In ExEx context, logs are typically provided through notifications
    //     // This method is a placeholder for the general structure
    //     Ok(())
    // }

    /// Processes a transaction receipt's logs
    pub async fn process_logs(
        &self,
        logs: &[Log],
        tx_hash: Option<String>,
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
            if log.address != abi::get_validator_staking_address() && !self.validator_manager.is_validator(&log.address).await {
                continue;
            }

            // Get the event type from the first topic
            if log.topics().is_empty() {
                warn!("Log has no topics");
                continue;
            }

            let event_topic = *log.topics().first().unwrap();

            // Route to appropriate handler based on event type
            if abi::is_validator_initialized_event(&event_topic.into()) {
                self.handle_validator_initialized_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            } else if abi::is_old_delegate_event(&event_topic.into()) {
                self.handle_old_delegate_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            } else if abi::is_old_undelegate_event(&event_topic.into()) {
                self.handle_old_undelegate_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            } else if abi::is_delegate_event(&event_topic.into()) {
                self.handle_delegate_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            } else if abi::is_undelegate_event(&event_topic.into()) {
                self.handle_undelegate_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            } else if abi::is_withdraw_commission_event(&event_topic.into()) {
                self.handle_withdraw_commission_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            } else if abi::is_withdraw_tip_fee_event(&event_topic.into()) {
                self.handle_withdraw_tip_fee_log(log, tx_hash.clone(), block_number, block_timestamp)
                    .await?;
            }
        }

        Ok(())
    }

    /// Handles ValidatorCreated event log
    async fn handle_validator_created_log(
        &self,
        log: &Log,
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
        let pubkey_hash_str = format!("0x{}", hex::encode(&pubkey_hash));
        let validator_address = Address::from_word(*log.topics().get(2).unwrap());

        // Note: In this case, pubkey is hashed in the topic
        // In actual implementation, we need to get the original pubkey from the transaction data
        // or keep a separate mapping

        self.event_handler
            .handle_validator_created(pubkey_hash_str, validator_address)
            .await?;

        Ok(())
    }

    /// Handles ValidatorInitialized event log
    async fn handle_validator_initialized_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing ValidatorInitialized event");

        // ValidatorInitialized(bytes indexed pubkey, address indexed validator, address operator, uint256 amount)
        // Topics: [event_signature, indexed_pubkey, indexed_validator]
        // Data: [operator, amount]

        if log.topics().len() < 3 {
            warn!("ValidatorInitialized log has insufficient topics");
            return Ok(());
        }

        // Extract indexed parameters from topics
        let pubkey_hash = *log.topics().get(1).unwrap();
        let pubkey_hash_str = format!("0x{}", hex::encode(&pubkey_hash));
        let validator_address = Address::from_word(*log.topics().get(2).unwrap());

        // Note: In this case, pubkey is hashed in the topic
        // In actual implementation, we need to get the original pubkey from the transaction data
        // or keep a separate mapping
        // 0x000000000000000000000000565e66aa2bcb27116937983f2f208efabf620ab200000000000000000000000000000000000000000000001b1ae4d6e2ef500000

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 64 {
            warn!("ValidatorInitialized log has insufficient data");
            return Ok(());
        }

        let operator_bytes: [u8; 20] = data[12..32].try_into()?;
        let amount_bytes: [u8; 32] = data[32..64].try_into()?;

        let operator_address = Address::from(operator_bytes);
        let amount = U256::from_be_bytes(amount_bytes);

        self.event_handler
            .handle_validator_initialized(
                pubkey_hash_str,
                validator_address,
                operator_address,
                amount.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles Delegate event log
    async fn handle_delegate_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
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

        let payer_address = Address::from_word(*log.topics().get(1).unwrap());
        let delegator_address = Address::from_word(*log.topics().get(2).unwrap());
        let validator_address = log.address;

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 64 {
            warn!("Delegate log has insufficient data");
            return Ok(());
        }

        let amount_bytes: [u8; 32] = data[0..32].try_into()?;
        let shares_bytes: [u8; 32] = data[32..64].try_into()?;

        let amount = U256::from_be_bytes(amount_bytes);
        let shares = U256::from_be_bytes(shares_bytes);

        self.event_handler
            .handle_delegate(
                payer_address,
                delegator_address,
                validator_address,
                amount.to_string(),
                shares.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles Undelegate event log
    async fn handle_undelegate_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
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

        let delegator_address = Address::from_word(*log.topics().get(1).unwrap());
        let withdrawal_address = Address::from_word(*log.topics().get(2).unwrap());
        let validator_address = log.address;

        // Decode data: shares and amount (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 64 {
            warn!("Undelegate log has insufficient data");
            return Ok(());
        }

        let shares_bytes: [u8; 32] = data[0..32].try_into()?;
        let amount_bytes: [u8; 32] = data[32..64].try_into()?;

        let shares = U256::from_be_bytes(shares_bytes);
        let amount = U256::from_be_bytes(amount_bytes);

        self.event_handler
            .handle_undelegate(
                delegator_address,
                withdrawal_address,
                validator_address,
                shares.to_string(),
                amount.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles Old Delegate event log
    async fn handle_old_delegate_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing Old Delegate event");

        // Delegate(address indexed delegatorAddress, uint amount, uint shares);
        // Topics: [event_signature, indexed_delegator]
        // Data: [amount, shares]

        if log.topics().len() < 2 {
            warn!("Old Delegate log has insufficient topics");
            return Ok(());
        }

        let delegator_address = Address::from_word(*log.topics().get(1).unwrap());
        let validator_address = log.address;

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 64 {
            warn!("Old Delegate log has insufficient data");
            return Ok(());
        }

        let amount_bytes: [u8; 32] = data[0..32].try_into()?;
        let shares_bytes: [u8; 32] = data[32..64].try_into()?;

        let amount = U256::from_be_bytes(amount_bytes);
        let shares = U256::from_be_bytes(shares_bytes);

        self.event_handler
            .handle_delegate(
                Address::ZERO,
                delegator_address,
                validator_address,
                amount.to_string(),
                shares.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles Old Undelegate event log
    async fn handle_old_undelegate_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing Old Undelegate event");

        // Undelegate(address indexed delegatorAddress, uint shares, uint amount)
        // Topics: [event_signature, indexed_delegator, indexed_withdrawal]
        // Data: [shares, amount]

        if log.topics().len() < 2 {
            warn!("Old Undelegate log has insufficient topics");
            return Ok(());
        }

        let delegator_address = Address::from_word(*log.topics().get(1).unwrap());
        let validator_address = log.address;

        // Decode data: shares and amount (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 64 {
            warn!("Old Undelegate log has insufficient data");
            return Ok(());
        }

        let shares_bytes: [u8; 32] = data[0..32].try_into()?;
        let amount_bytes: [u8; 32] = data[32..64].try_into()?;

        let shares = U256::from_be_bytes(shares_bytes);
        let amount = U256::from_be_bytes(amount_bytes);

        self.event_handler
            .handle_undelegate(
                delegator_address,
                Address::ZERO,
                validator_address,
                shares.to_string(),
                amount.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles WithdrawCommission event log
    async fn handle_withdraw_commission_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing WithdrawCommission event");

        // WithdrawCommission(address indexed ownerAddress, address indexed withdrawalAddress, uint amount);
        // Topics: [event_signature, indexed_owner, indexed_delegator]
        // Data: [amount]

        if log.topics().len() < 3 {
            warn!("WithdrawCommission log has insufficient topics");
            return Ok(());
        }

        let owner_address = Address::from_word(*log.topics().get(1).unwrap());
        let withdrawal_address = Address::from_word(*log.topics().get(2).unwrap());
        let validator_address = log.address;

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 32 {
            warn!("WithdrawCommission log has insufficient data");
            return Ok(());
        }

        let amount_bytes: [u8; 32] = data[0..32].try_into()?;

        let amount = U256::from_be_bytes(amount_bytes);

        self.event_handler
            .handle_withdraw_commission(
                owner_address,
                withdrawal_address,
                validator_address,
                amount.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }

    /// Handles WithdrawTipFee event log
    async fn handle_withdraw_tip_fee_log(
        &self,
        log: &Log,
        tx_hash: Option<String>,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        debug!("Processing WithdrawTipFee event");

        // WithdrawTipFee(address indexed ownerAddress, address indexed withdrawalAddress, uint amount);
        // Topics: [event_signature, indexed_owner, indexed_delegator]
        // Data: [amount]

        if log.topics().len() < 3 {
            warn!("WithdrawTipFee log has insufficient topics");
            return Ok(());
        }

        let owner_address = Address::from_word(*log.topics().get(1).unwrap());
        let withdrawal_address = Address::from_word(*log.topics().get(2).unwrap());
        let validator_address = log.address;

        // Decode data: amount and shares (each 32 bytes)
        let data = log.data.data.as_ref();
        if data.len() < 32 {
            warn!("WithdrawTipFee log has insufficient data");
            return Ok(());
        }

        let amount_bytes: [u8; 32] = data[0..32].try_into()?;

        let amount = U256::from_be_bytes(amount_bytes);

        self.event_handler
            .handle_withdraw_tip_fee(
                owner_address,
                withdrawal_address,
                validator_address,
                amount.to_string(),
                tx_hash,
                Some(block_number),
                block_timestamp,
            )
            .await?;

        Ok(())
    }
}