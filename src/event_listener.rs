// Event listener module for monitoring and processing blockchain events
// Extracts events from blocks and routes them to appropriate handlers

use crate::abi;
use crate::event_handler::EventHandler;
use crate::validators::ValidatorManager;
use alloy_primitives::{Address, Log, U256};
use eyre::Result;
use std::sync::Arc;
use tracing::{debug, warn};

/// A staking event decoded from a raw log, independent of how the log was sourced.
/// Shared by the live ExEx path (`EventListener::process_logs`) and the standalone
/// backfill tool, so both decode logs identically.
#[derive(Debug, Clone)]
pub enum DecodedEvent {
    ValidatorInitialized {
        pubkey_hash: String,
        validator_address: Address,
        operator_address: Address,
        amount: String,
    },
    Delegate {
        payer_address: Address,
        delegator_address: Address,
        validator_address: Address,
        amount: String,
        shares: String,
    },
    Undelegate {
        delegator_address: Address,
        withdrawal_address: Address,
        validator_address: Address,
        shares: String,
        amount: String,
    },
    WithdrawCommission {
        owner_address: Address,
        withdrawal_address: Address,
        validator_address: Address,
        amount: String,
    },
    WithdrawTipFee {
        owner_address: Address,
        withdrawal_address: Address,
        validator_address: Address,
        amount: String,
    },
}

/// Decodes a raw log into a `DecodedEvent` based on its first topic. Returns `None` for
/// logs that don't match a known staking event signature, or whose topics/data don't
/// match the expected shape for their signature.
pub fn decode_log(log: &Log) -> Option<DecodedEvent> {
    let event_topic = *log.topics().first()?;

    if abi::is_validator_initialized_event(&event_topic.into()) {
        decode_validator_initialized(log)
    } else if abi::is_old_delegate_event(&event_topic.into()) {
        decode_old_delegate(log)
    } else if abi::is_delegate_event(&event_topic.into()) {
        decode_delegate(log)
    } else if abi::is_old_undelegate_event(&event_topic.into()) {
        decode_old_undelegate(log)
    } else if abi::is_undelegate_event(&event_topic.into()) {
        decode_undelegate(log)
    } else if abi::is_withdraw_commission_event(&event_topic.into()) {
        decode_withdraw_commission(log)
    } else if abi::is_withdraw_tip_fee_event(&event_topic.into()) {
        decode_withdraw_tip_fee(log)
    } else {
        None
    }
}

/// ValidatorInitialized(bytes indexed pubkey, address indexed validator, address operator, uint256 amount)
fn decode_validator_initialized(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 3 {
        warn!("ValidatorInitialized log has insufficient topics");
        return None;
    }

    let pubkey_hash = *log.topics().get(1)?;
    let pubkey_hash_str = format!("0x{}", hex::encode(pubkey_hash));
    let validator_address = Address::from_word(*log.topics().get(2)?);

    let data = log.data.data.as_ref();
    if data.len() < 64 {
        warn!("ValidatorInitialized log has insufficient data");
        return None;
    }
    let operator_bytes: [u8; 20] = data[12..32].try_into().ok()?;
    let amount_bytes: [u8; 32] = data[32..64].try_into().ok()?;
    let operator_address = Address::from(operator_bytes);
    let amount = U256::from_be_bytes(amount_bytes);

    Some(DecodedEvent::ValidatorInitialized {
        pubkey_hash: pubkey_hash_str,
        validator_address,
        operator_address,
        amount: amount.to_string(),
    })
}

/// Delegate(address indexed payerAddress, address indexed delegatorAddress, uint amount, uint shares)
fn decode_delegate(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 3 {
        warn!("Delegate log has insufficient topics");
        return None;
    }

    let payer_address = Address::from_word(*log.topics().get(1)?);
    let delegator_address = Address::from_word(*log.topics().get(2)?);
    let validator_address = log.address;

    let data = log.data.data.as_ref();
    if data.len() < 64 {
        warn!("Delegate log has insufficient data");
        return None;
    }
    let amount_bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let shares_bytes: [u8; 32] = data[32..64].try_into().ok()?;

    Some(DecodedEvent::Delegate {
        payer_address,
        delegator_address,
        validator_address,
        amount: U256::from_be_bytes(amount_bytes).to_string(),
        shares: U256::from_be_bytes(shares_bytes).to_string(),
    })
}

/// Delegate(address indexed delegatorAddress, uint amount, uint shares) -- pre-upgrade signature
fn decode_old_delegate(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 2 {
        warn!("Old Delegate log has insufficient topics");
        return None;
    }

    let delegator_address = Address::from_word(*log.topics().get(1)?);
    let validator_address = log.address;

    let data = log.data.data.as_ref();
    if data.len() < 64 {
        warn!("Old Delegate log has insufficient data");
        return None;
    }
    let amount_bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let shares_bytes: [u8; 32] = data[32..64].try_into().ok()?;

    Some(DecodedEvent::Delegate {
        payer_address: Address::ZERO,
        delegator_address,
        validator_address,
        amount: U256::from_be_bytes(amount_bytes).to_string(),
        shares: U256::from_be_bytes(shares_bytes).to_string(),
    })
}

/// Undelegate(address indexed delegatorAddress, address indexed withdrawalAddress, uint shares, uint amount)
fn decode_undelegate(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 3 {
        warn!("Undelegate log has insufficient topics");
        return None;
    }

    let delegator_address = Address::from_word(*log.topics().get(1)?);
    let withdrawal_address = Address::from_word(*log.topics().get(2)?);
    let validator_address = log.address;

    let data = log.data.data.as_ref();
    if data.len() < 64 {
        warn!("Undelegate log has insufficient data");
        return None;
    }
    let shares_bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let amount_bytes: [u8; 32] = data[32..64].try_into().ok()?;

    Some(DecodedEvent::Undelegate {
        delegator_address,
        withdrawal_address,
        validator_address,
        shares: U256::from_be_bytes(shares_bytes).to_string(),
        amount: U256::from_be_bytes(amount_bytes).to_string(),
    })
}

/// Undelegate(address indexed delegatorAddress, uint shares, uint amount) -- pre-upgrade signature
fn decode_old_undelegate(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 2 {
        warn!("Old Undelegate log has insufficient topics");
        return None;
    }

    let delegator_address = Address::from_word(*log.topics().get(1)?);
    let validator_address = log.address;

    let data = log.data.data.as_ref();
    if data.len() < 64 {
        warn!("Old Undelegate log has insufficient data");
        return None;
    }
    let shares_bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let amount_bytes: [u8; 32] = data[32..64].try_into().ok()?;

    Some(DecodedEvent::Undelegate {
        delegator_address,
        withdrawal_address: Address::ZERO,
        validator_address,
        shares: U256::from_be_bytes(shares_bytes).to_string(),
        amount: U256::from_be_bytes(amount_bytes).to_string(),
    })
}

/// WithdrawCommission(address indexed ownerAddress, address indexed withdrawalAddress, uint amount)
fn decode_withdraw_commission(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 3 {
        warn!("WithdrawCommission log has insufficient topics");
        return None;
    }

    let owner_address = Address::from_word(*log.topics().get(1)?);
    let withdrawal_address = Address::from_word(*log.topics().get(2)?);
    let validator_address = log.address;

    let data = log.data.data.as_ref();
    if data.len() < 32 {
        warn!("WithdrawCommission log has insufficient data");
        return None;
    }
    let amount_bytes: [u8; 32] = data[0..32].try_into().ok()?;

    Some(DecodedEvent::WithdrawCommission {
        owner_address,
        withdrawal_address,
        validator_address,
        amount: U256::from_be_bytes(amount_bytes).to_string(),
    })
}

/// WithdrawTipFee(address indexed ownerAddress, address indexed withdrawalAddress, uint amount)
fn decode_withdraw_tip_fee(log: &Log) -> Option<DecodedEvent> {
    if log.topics().len() < 3 {
        warn!("WithdrawTipFee log has insufficient topics");
        return None;
    }

    let owner_address = Address::from_word(*log.topics().get(1)?);
    let withdrawal_address = Address::from_word(*log.topics().get(2)?);
    let validator_address = log.address;

    let data = log.data.data.as_ref();
    if data.len() < 32 {
        warn!("WithdrawTipFee log has insufficient data");
        return None;
    }
    let amount_bytes: [u8; 32] = data[0..32].try_into().ok()?;

    Some(DecodedEvent::WithdrawTipFee {
        owner_address,
        withdrawal_address,
        validator_address,
        amount: U256::from_be_bytes(amount_bytes).to_string(),
    })
}

/// Applies a decoded event to the database via the given event handler. Shared by the
/// live ExEx path and the backfill tool.
pub async fn apply_decoded_event(
    event_handler: &EventHandler,
    event: DecodedEvent,
    tx_hash: Option<String>,
    block_number: u64,
    block_timestamp: Option<i64>,
) -> Result<()> {
    match event {
        DecodedEvent::ValidatorInitialized { pubkey_hash, validator_address, operator_address, amount } => {
            event_handler
                .handle_validator_initialized(
                    pubkey_hash,
                    validator_address,
                    operator_address,
                    amount,
                    tx_hash,
                    Some(block_number),
                    block_timestamp,
                )
                .await
        }
        DecodedEvent::Delegate { payer_address, delegator_address, validator_address, amount, shares } => {
            event_handler
                .handle_delegate(
                    payer_address,
                    delegator_address,
                    validator_address,
                    amount,
                    shares,
                    tx_hash,
                    Some(block_number),
                    block_timestamp,
                )
                .await
        }
        DecodedEvent::Undelegate { delegator_address, withdrawal_address, validator_address, shares, amount } => {
            event_handler
                .handle_undelegate(
                    delegator_address,
                    withdrawal_address,
                    validator_address,
                    shares,
                    amount,
                    tx_hash,
                    Some(block_number),
                    block_timestamp,
                )
                .await
        }
        DecodedEvent::WithdrawCommission { owner_address, withdrawal_address, validator_address, amount } => {
            event_handler
                .handle_withdraw_commission(
                    owner_address,
                    withdrawal_address,
                    validator_address,
                    amount,
                    tx_hash,
                    Some(block_number),
                    block_timestamp,
                )
                .await
        }
        DecodedEvent::WithdrawTipFee { owner_address, withdrawal_address, validator_address, amount } => {
            event_handler
                .handle_withdraw_tip_fee(
                    owner_address,
                    withdrawal_address,
                    validator_address,
                    amount,
                    tx_hash,
                    Some(block_number),
                    block_timestamp,
                )
                .await
        }
    }
}

/// Processes blocks in the live ExEx path
pub struct EventListener {
    event_handler: Arc<EventHandler>,
    validator_manager: Arc<ValidatorManager>,
}

impl EventListener {
    /// Creates a new event listener
    pub fn new(event_handler: Arc<EventHandler>, validator_manager: Arc<ValidatorManager>) -> Self {
        Self { event_handler, validator_manager }
    }

    /// Processes a transaction receipt's logs
    pub async fn process_logs(
        &self,
        logs: &[Log],
        tx_hash: Option<String>,
        block_number: u64,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        for log in logs {
            debug!("Processing log from address: {}, topics: {}", log.address, log.topics().len());

            // Check if this log is from the ValidatorStaking contract or a known validator
            if log.address != abi::get_validator_staking_address() && !self.validator_manager.is_validator(&log.address).await {
                continue;
            }

            if log.topics().is_empty() {
                warn!("Log has no topics");
                continue;
            }

            if let Some(event) = decode_log(log) {
                apply_decoded_event(&self.event_handler, event, tx_hash.clone(), block_number, block_timestamp).await?;
            }
        }

        Ok(())
    }
}
