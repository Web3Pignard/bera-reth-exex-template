// Event handler module for processing staking-related events
// Handles ValidatorCreated, Delegate, Undelegate, and Redelegate events

use crate::db::{Database, StakingEvent};
use crate::validators::ValidatorManager;
use alloy_primitives::Address;
use eyre::Result;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Processes staking events and updates the database
pub struct EventHandler {
    db: Arc<Database>,
    validator_manager: Arc<ValidatorManager>,
}

impl EventHandler {
    /// Creates a new event handler
    pub fn new(db: Arc<Database>, validator_manager: Arc<ValidatorManager>) -> Self {
        Self {
            db,
            validator_manager,
        }
    }

    /// Handles ValidatorCreated event
    /// Adds a new validator to the system
    pub async fn handle_validator_created(
        &self,
        pubkey: Vec<u8>,
        validator_address: Address,
    ) -> Result<()> {
        // Convert pubkey bytes to hex string (98 chars = 0x + 96 hex digits = 48 bytes)
        let pubkey_str = format!("0x{}", hex::encode(&pubkey));

        if pubkey_str.len() != 98 {
            warn!(
                "Invalid pubkey length: {} (expected 98)",
                pubkey_str.len()
            );
            return Ok(());
        }

        info!(
            "ValidatorCreated event: pubkey={}, address={}",
            pubkey_str, validator_address
        );

        // Add to database
        self.db
            .upsert_validator(pubkey_str.clone(), validator_address)?;

        // Update in-memory validator list
        self.validator_manager
            .add_validator(pubkey_str, validator_address)
            .await?;

        info!("Validator successfully registered: {}", validator_address);
        Ok(())
    }

    /// Handles Delegate event
    /// Creates an event record and updates delegator's total_delegated amount
    pub async fn handle_delegate(
        &self,
        payer_address: Address,
        delegator_address: Address,
        validator_address: Address,
        amount: String,
        shares: String,
        tx_hash: Option<String>,
        block_number: Option<u64>,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        info!(
            "Delegate event: delegator={}, validator={}, amount={}",
            delegator_address, validator_address, amount
        );

        // Create event record
        let event = StakingEvent {
            delegator_address,
            validator_address,
            event_type: 0, // Delegate
            amount: amount.clone(),
            shares,
            transaction_hash: tx_hash,
            block_number,
            block_timestamp,
        };

        // Insert event
        self.db.insert_event(&event)?;

        // Get or create delegator record and update total_delegated
        match self.db.get_delegator(&delegator_address)? {
            Some(delegator) => {
                // Calculate new total
                let current: i128 = delegator.total_delegated.parse().unwrap_or(0);
                let amount_val: i128 = amount.parse().unwrap_or(0);
                let new_total = (current + amount_val).to_string();

                self.db
                    .upsert_delegator(&delegator_address, &new_total, &delegator.total_undelegated)?;
            }
            None => {
                // Create new delegator record
                self.db
                    .upsert_delegator(&delegator_address, &amount, "0")?;
            }
        }

        debug!("Delegate event processed successfully");
        Ok(())
    }

    /// Handles Undelegate event
    /// Creates an event record and updates delegator's total_undelegated amount
    pub async fn handle_undelegate(
        &self,
        delegator_address: Address,
        withdrawal_address: Address,
        validator_address: Address,
        shares: String,
        amount: String,
        tx_hash: Option<String>,
        block_number: Option<u64>,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        info!(
            "Undelegate event: delegator={}, validator={}, amount={}",
            delegator_address, validator_address, amount
        );

        // Create event record (event_type = 1 for Undelegate)
        let event = StakingEvent {
            delegator_address,
            validator_address,
            event_type: 1, // Undelegate
            amount: amount.clone(),
            shares,
            transaction_hash: tx_hash,
            block_number,
            block_timestamp,
        };

        // Insert event
        self.db.insert_event(&event)?;

        // Get or create delegator record and update total_undelegated
        match self.db.get_delegator(&delegator_address)? {
            Some(delegator) => {
                // Calculate new total
                let current: i128 = delegator.total_undelegated.parse().unwrap_or(0);
                let amount_val: i128 = amount.parse().unwrap_or(0);
                let new_total = (current + amount_val).to_string();

                self.db
                    .upsert_delegator(&delegator_address, &delegator.total_delegated, &new_total)?;
            }
            None => {
                // Create new delegator record with undelegated amount
                self.db
                    .upsert_delegator(&delegator_address, "0", &amount)?;
            }
        }

        debug!("Undelegate event processed successfully");
        Ok(())
    }

    /// Handles Redelegate event
    /// Creates an event record with event_type = 2
    pub async fn handle_redelegate(
        &self,
        owner_address: Address,
        validator_address: Address,
        amount: String,
        shares: String,
        tx_hash: Option<String>,
        block_number: Option<u64>,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        info!(
            "Redelegate event: owner={}, validator={}, amount={}",
            owner_address, validator_address, amount
        );

        // Create event record (event_type = 2 for Redelegate)
        let event = StakingEvent {
            delegator_address: owner_address,
            validator_address,
            event_type: 2, // Redelegate
            amount,
            shares,
            transaction_hash: tx_hash,
            block_number,
            block_timestamp,
        };

        // Insert event
        self.db.insert_event(&event)?;

        debug!("Redelegate event processed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_handler_creation() {
        // This is a placeholder test structure
        // In actual implementation, you would test event handling logic
    }
}
