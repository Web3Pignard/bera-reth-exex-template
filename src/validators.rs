// Validator management module for the Berachain staking indexer
// Handles tracking and updating the list of active validators

use crate::db::Database;
use alloy_primitives::Address;
use eyre::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Manages the list of active validators
/// Keeps an in-memory cache that gets updated when validators are added
pub struct ValidatorManager {
    db: Arc<Database>,
    validator_addresses: Arc<RwLock<Vec<Address>>>,
}

impl ValidatorManager {
    /// Creates a new validator manager and loads validators from database
    pub fn new(db: Arc<Database>) -> Result<Self> {
        let validator_addresses = db.get_all_validator_addresses()?;
        
        info!("ValidatorManager initialized with {} validators", validator_addresses.len());

        Ok(Self {
            db,
            validator_addresses: Arc::new(RwLock::new(validator_addresses)),
        })
    }

    /// Gets the current list of validator addresses
    pub async fn get_validator_addresses(&self) -> Vec<Address> {
        self.validator_addresses.read().await.clone()
    }

    /// Adds a new validator and updates the in-memory list
    pub async fn add_validator(&self, pubkey: String, address: Address) -> Result<()> {
        // Insert into database
        self.db.upsert_validator(pubkey.clone(), address)?;

        // Update in-memory list if not already present
        let mut addresses = self.validator_addresses.write().await;
        if !addresses.contains(&address) {
            addresses.push(address);
            info!(
                "New validator added: {} (total: {})",
                address,
                addresses.len()
            );
        }

        Ok(())
    }

    /// Refreshes the validator list from the database
    /// This should be called periodically or when validators change
    pub async fn refresh(&self) -> Result<()> {
        let validator_addresses = self.db.get_all_validator_addresses()?;
        let mut addresses = self.validator_addresses.write().await;
        
        debug!(
            "Refreshing validators: old count={}, new count={}",
            addresses.len(),
            validator_addresses.len()
        );

        *addresses = validator_addresses;
        Ok(())
    }

    /// Gets the count of active validators
    pub async fn validator_count(&self) -> usize {
        self.validator_addresses.read().await.len()
    }

    /// Checks if an address is a known validator
    pub async fn is_validator(&self, address: &Address) -> bool {
        self.validator_addresses.read().await.contains(address)
    }
}
