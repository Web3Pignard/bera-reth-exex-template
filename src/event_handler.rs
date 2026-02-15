// Event handler module for processing staking-related events
// Handles ValidatorCreated, Delegate, Undelegate, and WithdrawCommission events

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
        pubkey_hash_str: String,
        validator_address: Address,
    ) -> Result<()> {
        info!(
            "ValidatorCreated event: pubkey_hash={}, address={}",
            pubkey_hash_str, validator_address
        );

        // Add to database
        self.db
            .upsert_validator(pubkey_hash_str.clone(), validator_address)?;

        // Update in-memory validator list
        self.validator_manager
            .add_validator(pubkey_hash_str, validator_address)
            .await?;

        info!("Validator successfully registered: {}", validator_address);
        Ok(())
    }

    /// Handles ValidatorInitialized event
    /// Adds a new validator to the system
    pub async fn handle_validator_initialized(
        &self,
        pubkey_hash_str: String,
        validator_address: Address,
        operator_address: Address,
        amount: String,
        tx_hash: Option<String>,
        block_number: Option<u64>,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        info!(
            "ValidatorInitialized event: pubkey_hash={}, address={}, validator={}, amount={}",
            pubkey_hash_str, validator_address, operator_address, amount,
        );

        // Add to database
        self.db
            .upsert_validator(pubkey_hash_str.clone(), validator_address)?;

        // Update in-memory validator list
        self.validator_manager
            .add_validator(pubkey_hash_str, validator_address)
            .await?;

        info!("Validator successfully registered: {}", validator_address);
        
        // Create event record
        let event = StakingEvent {
            delegator_address: operator_address,
            validator_address,
            event_type: 0, // Delegate
            amount: amount.clone(),
            shares: amount.clone(),
            transaction_hash: tx_hash,
            block_number,
            block_timestamp,
        };

        // Insert event
        self.db.insert_event(&event)?;

        // Get or create delegator record and update total_delegated
        match self.db.get_delegator(&operator_address)? {
            Some(delegator) => {
                // Calculate new total
                let current: i128 = delegator.total_delegated.parse().unwrap_or(0);
                let amount_val: i128 = amount.parse().unwrap_or(0);
                let new_total = (current + amount_val).to_string();

                self.db
                    .upsert_delegator(&operator_address, &new_total, &delegator.total_undelegated)?;
            }
            None => {
                // Create new delegator record
                self.db
                    .upsert_delegator(&operator_address, &amount, "0")?;
            }
        }

        debug!("Delegate event processed successfully");
        
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

    /// Handles WithdrawCommission event
    /// Creates an event record with event_type = 2
    pub async fn handle_withdraw_commission(
        &self,
        owner_address: Address,
        withdrawal_address: Address,
        validator_address: Address,
        amount: String,
        tx_hash: Option<String>,
        block_number: Option<u64>,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        info!(
            "WithdrawCommission event: owner={}, validator={}, amount={}",
            owner_address, validator_address, amount
        );

        // Create event record (event_type = 2 for WithdrawCommission)
        let event = StakingEvent {
            delegator_address: owner_address,
            validator_address,
            event_type: 2, // WithdrawCommission
            amount: amount.clone(),
            shares: String::from("0"),
            transaction_hash: tx_hash,
            block_number,
            block_timestamp,
        };

        // Insert event
        self.db.insert_event(&event)?;

        // Get or create delegator record and update total_undelegated
        match self.db.get_delegator(&owner_address)? {
            Some(delegator) => {
                // Calculate new total
                let current: i128 = delegator.total_undelegated.parse().unwrap_or(0);
                let amount_val: i128 = amount.parse().unwrap_or(0);
                let new_total = (current + amount_val).to_string();

                self.db
                    .upsert_delegator(&owner_address, &delegator.total_delegated, &new_total)?;
            }
            None => {
                // Create new delegator record with undelegated amount
                self.db
                    .upsert_delegator(&owner_address, "0", &amount)?;
            }
        }

        debug!("WithdrawCommission event processed successfully");
        Ok(())
    }

    /// Handles WithdrawTipFee event
    /// Creates an event record with event_type = 3
    pub async fn handle_withdraw_tip_fee(
        &self,
        owner_address: Address,
        withdrawal_address: Address,
        validator_address: Address,
        amount: String,
        tx_hash: Option<String>,
        block_number: Option<u64>,
        block_timestamp: Option<i64>,
    ) -> Result<()> {
        info!(
            "WithdrawTipFee event: owner={}, validator={}, amount={}",
            owner_address, validator_address, amount
        );

        // Create event record (event_type = 3 for WithdrawTipFee)
        let event = StakingEvent {
            delegator_address: owner_address,
            validator_address,
            event_type: 3, // WithdrawTipFee
            amount: amount.clone(),
            shares: String::from("0"),
            transaction_hash: tx_hash,
            block_number,
            block_timestamp,
        };

        // Insert event
        self.db.insert_event(&event)?;

        // Get or create delegator record and update total_undelegated
        match self.db.get_delegator(&owner_address)? {
            Some(delegator) => {
                // Calculate new total
                let current: i128 = delegator.total_undelegated.parse().unwrap_or(0);
                let amount_val: i128 = amount.parse().unwrap_or(0);
                let new_total = (current + amount_val).to_string();

                self.db
                    .upsert_delegator(&owner_address, &delegator.total_delegated, &new_total)?;
            }
            None => {
                // Create new delegator record with undelegated amount
                self.db
                    .upsert_delegator(&owner_address, "0", &amount)?;
            }
        }

        debug!("WithdrawTipFee event processed successfully");
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
