// Database operations module for the 0gchain staking indexer
// Handles SQLite operations for validators, delegators, and events

use alloy_primitives::Address;
use eyre::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;
use tracing::{debug, error, info};

/// Represents a validator in the staking system
#[derive(Debug, Clone)]
pub struct Validator {
    pub pubkey: String,
    pub address: Address,
}

/// Represents a delegator's delegation state
#[derive(Debug, Clone)]
pub struct Delegator {
    pub address: Address,
    pub total_delegated: String,
    pub total_undelegated: String,
}

/// Represents a validator's operator/owner commission and tip fee withdrawals
#[derive(Debug, Clone)]
pub struct ValidatorOperator {
    pub validator_address: Address,
    pub owner_address: Address,
    pub total_commission_withdrawn: String,
    pub total_tip_fee_withdrawn: String,
}

/// Represents a staking event (Delegate, Undelegate, or WithdrawCommission)
#[derive(Debug, Clone)]
pub struct StakingEvent {
    pub delegator_address: Address,
    pub validator_address: Address,
    pub event_type: u8, // 0: Delegate, 1: Undelegate, 2: WithdrawCommission, 3: WithdrawTipFee
    pub amount: String,
    pub shares: String,
    pub transaction_hash: Option<String>,
    pub block_number: Option<u64>,
    pub block_timestamp: Option<i64>,
}

/// Database connection wrapper with thread-safe access
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Creates a new database connection and initializes tables
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| eyre::eyre!("Failed to open database: {}", e))?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| eyre::eyre!("Failed to enable foreign keys: {}", e))?;

        // Initialize schema
        conn.execute_batch(include_str!("../schema.sql"))
            .map_err(|e| eyre::eyre!("Failed to initialize schema: {}", e))?;

        info!("Database initialized successfully at {}", db_path);

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ============ Validator Operations ============

    /// Inserts a new validator or updates if already exists
    pub fn upsert_validator(&self, pubkey: String, address: Address) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        conn.execute(
            "INSERT INTO validators (pubkey, address, created_at, updated_at) 
             VALUES (?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(address) DO UPDATE SET 
             pubkey = excluded.pubkey,
             updated_at = CURRENT_TIMESTAMP",
            params![&pubkey, &address_str],
        ).map_err(|e| eyre::eyre!("Failed to upsert validator: {}", e))?;

        debug!("Validator inserted/updated: {} ({})", pubkey, address);
        Ok(())
    }

    /// Retrieves all validator addresses
    pub fn get_all_validator_addresses(&self) -> Result<Vec<Address>> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT address FROM validators")
            .map_err(|e| eyre::eyre!("Failed to prepare statement: {}", e))?;

        let addresses = stmt
            .query_map([], |row| {
                let address_str: String = row.get(0)?;
                Ok(Address::parse_checksummed(&address_str, None).unwrap_or_default())
            })
            .map_err(|e| eyre::eyre!("Failed to query validators: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| eyre::eyre!("Failed to collect validator addresses: {}", e))?;

        debug!("Retrieved {} validator addresses", addresses.len());
        Ok(addresses)
    }

    /// Retrieves validator by address
    pub fn get_validator(&self, address: &Address) -> Result<Option<Validator>> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        let validator = conn
            .query_row(
                "SELECT pubkey, address FROM validators WHERE address = ?",
                params![&address_str],
                |row| {
                    Ok(Validator {
                        pubkey: row.get(0)?,
                        address: Address::parse_checksummed(&row.get::<_, String>(1)?, None)
                            .unwrap_or_default(),
                    })
                },
            )
            .optional()
            .map_err(|e| eyre::eyre!("Failed to query validator: {}", e))?;

        Ok(validator)
    }

    // ============ Delegator Operations ============

    /// Inserts or updates a delegator
    pub fn upsert_delegator(&self, address: &Address, total_delegated: &str, total_undelegated: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        conn.execute(
            "INSERT INTO delegators (address, total_delegated, total_undelegated, created_at, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(address) DO UPDATE SET
             total_delegated = excluded.total_delegated,
             total_undelegated = excluded.total_undelegated,
             updated_at = CURRENT_TIMESTAMP",
            params![&address_str, total_delegated, total_undelegated],
        ).map_err(|e| eyre::eyre!("Failed to upsert delegator: {}", e))?;

        debug!("Delegator upserted: {} (delegated: {}, undelegated: {})", address, total_delegated, total_undelegated);
        Ok(())
    }

    /// Retrieves a delegator by address
    pub fn get_delegator(&self, address: &Address) -> Result<Option<Delegator>> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        let delegator = conn
            .query_row(
                "SELECT address, total_delegated, total_undelegated FROM delegators WHERE address = ?",
                params![&address_str],
                |row| {
                    Ok(Delegator {
                        address: Address::parse_checksummed(&row.get::<_, String>(0)?, None)
                            .unwrap_or_default(),
                        total_delegated: row.get(1)?,
                        total_undelegated: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| eyre::eyre!("Failed to query delegator: {}", e))?;

        Ok(delegator)
    }

    /// Increments delegator's total_delegated amount
    pub fn add_delegated_amount(&self, address: &Address, amount: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        // Parse amounts as integers for addition
        let current = conn
            .query_row(
                "SELECT total_delegated FROM delegators WHERE address = ?",
                params![&address_str],
                |row| {
                    let total_delegated: String = row.get(0)?;
                    Ok(total_delegated.parse().unwrap_or(0i128))
                }
            )
            .map_err(|e| eyre::eyre!("Failed to query total_delegated: {}", e))?;

        let amount_val: i128 = amount.parse().unwrap_or(0);
        let new_total = (current + amount_val).to_string();

        conn.execute(
            "UPDATE delegators 
             SET total_delegated = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE address = ?",
            params![&new_total, &address_str],
        ).map_err(|e| eyre::eyre!("Failed to add delegated amount: {}", e))?;

        debug!("Added delegated amount {} to delegator {}", amount, address);
        Ok(())
    }

    /// Increments delegator's total_undelegated amount
    pub fn add_undelegated_amount(&self, address: &Address, amount: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        // Parse amounts as integers for addition
        let current = conn
            .query_row(
                "SELECT total_undelegated FROM delegators WHERE address = ?",
                params![&address_str],
                |row| {
                    let total_undelegated: String = row.get(0)?;
                    Ok(total_undelegated.parse().unwrap_or(0i128))
                }
            )
            .map_err(|e| eyre::eyre!("Failed to query total_delegated: {}", e))?;

        let amount_val: i128 = amount.parse().unwrap_or(0);
        let new_total = (current + amount_val).to_string();

        conn.execute(
            "UPDATE delegators 
             SET total_undelegated = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE address = ?",
            params![&new_total, &address_str],
        ).map_err(|e| eyre::eyre!("Failed to add undelegated amount: {}", e))?;

        debug!("Added undelegated amount {} to delegator {}", amount, address);
        Ok(())
    }

    // ============ Validator Operator Operations ============

    /// Retrieves a validator operator's withdrawal totals
    pub fn get_validator_operator(
        &self,
        validator_address: &Address,
        owner_address: &Address,
    ) -> Result<Option<ValidatorOperator>> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let validator_str = validator_address.to_checksum(None);
        let owner_str = owner_address.to_checksum(None);

        let validator_operator = conn
            .query_row(
                "SELECT validator_address, owner_address, total_commission_withdrawn, total_tip_fee_withdrawn
                 FROM validator_operator WHERE validator_address = ? AND owner_address = ?",
                params![&validator_str, &owner_str],
                |row| {
                    Ok(ValidatorOperator {
                        validator_address: Address::parse_checksummed(&row.get::<_, String>(0)?, None)
                            .unwrap_or_default(),
                        owner_address: Address::parse_checksummed(&row.get::<_, String>(1)?, None)
                            .unwrap_or_default(),
                        total_commission_withdrawn: row.get(2)?,
                        total_tip_fee_withdrawn: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|e| eyre::eyre!("Failed to query validator operator: {}", e))?;

        Ok(validator_operator)
    }

    /// Inserts or updates a validator operator's withdrawal totals
    pub fn upsert_validator_operator(
        &self,
        validator_address: &Address,
        owner_address: &Address,
        total_commission_withdrawn: &str,
        total_tip_fee_withdrawn: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let validator_str = validator_address.to_checksum(None);
        let owner_str = owner_address.to_checksum(None);

        conn.execute(
            "INSERT INTO validator_operator (validator_address, owner_address, total_commission_withdrawn, total_tip_fee_withdrawn, created_at, updated_at)
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(validator_address, owner_address) DO UPDATE SET
             total_commission_withdrawn = excluded.total_commission_withdrawn,
             total_tip_fee_withdrawn = excluded.total_tip_fee_withdrawn,
             updated_at = CURRENT_TIMESTAMP",
            params![&validator_str, &owner_str, total_commission_withdrawn, total_tip_fee_withdrawn],
        ).map_err(|e| eyre::eyre!("Failed to upsert validator operator: {}", e))?;

        debug!(
            "Validator operator upserted: validator={}, owner={} (commission: {}, tip_fee: {})",
            validator_address, owner_address, total_commission_withdrawn, total_tip_fee_withdrawn
        );
        Ok(())
    }

    // ============ Event Operations ============

    /// Inserts a staking event
    pub fn insert_event(&self, event: &StakingEvent) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let delegator_str = event.delegator_address.to_checksum(None);
        let validator_str = event.validator_address.to_checksum(None);

        conn.execute(
            "INSERT INTO events (delegator_address, validator_address, event_type, amount, shares, 
                                 transaction_hash, block_number, block_timestamp, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
            params![
                &delegator_str,
                &validator_str,
                event.event_type,
                &event.amount,
                &event.shares,
                &event.transaction_hash,
                event.block_number.map(|n| n as i64),
                event.block_timestamp
            ],
        ).map_err(|e| eyre::eyre!("Failed to insert event: {}", e))?;

        debug!(
            "Event inserted: type={}, delegator={}, validator={}",
            event.event_type, event.delegator_address, event.validator_address
        );
        Ok(())
    }

    /// Retrieves all events for a delegator
    pub fn get_delegator_events(&self, address: &Address) -> Result<Vec<StakingEvent>> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        let mut stmt = conn
            .prepare(
                "SELECT delegator_address, validator_address, event_type, amount, shares, 
                        transaction_hash, block_number, block_timestamp 
                 FROM events WHERE delegator_address = ? 
                 ORDER BY created_at DESC"
            )
            .map_err(|e| eyre::eyre!("Failed to prepare statement: {}", e))?;

        let events = stmt
            .query_map(params![&address_str], |row| {
                Ok(StakingEvent {
                    delegator_address: Address::parse_checksummed(&row.get::<_, String>(0)?, None)
                        .unwrap_or_default(),
                    validator_address: Address::parse_checksummed(&row.get::<_, String>(1)?, None)
                        .unwrap_or_default(),
                    event_type: row.get(2)?,
                    amount: row.get(3)?,
                    shares: row.get(4)?,
                    transaction_hash: row.get(5)?,
                    block_number: row.get::<_, Option<i64>>(6)?.map(|n| n as u64),
                    block_timestamp: row.get(7)?,
                })
            })
            .map_err(|e| eyre::eyre!("Failed to query events: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| eyre::eyre!("Failed to collect events: {}", e))?;

        debug!("Retrieved {} events for delegator {}", events.len(), address);
        Ok(events)
    }

    /// Retrieves all events for a validator
    pub fn get_validator_events(&self, address: &Address) -> Result<Vec<StakingEvent>> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = address.to_checksum(None);

        let mut stmt = conn
            .prepare(
                "SELECT delegator_address, validator_address, event_type, amount, shares, 
                        transaction_hash, block_number, block_timestamp 
                 FROM events WHERE validator_address = ? 
                 ORDER BY created_at DESC"
            )
            .map_err(|e| eyre::eyre!("Failed to prepare statement: {}", e))?;

        let events = stmt
            .query_map(params![&address_str], |row| {
                Ok(StakingEvent {
                    delegator_address: Address::parse_checksummed(&row.get::<_, String>(0)?, None)
                        .unwrap_or_default(),
                    validator_address: Address::parse_checksummed(&row.get::<_, String>(1)?, None)
                        .unwrap_or_default(),
                    event_type: row.get(2)?,
                    amount: row.get(3)?,
                    shares: row.get(4)?,
                    transaction_hash: row.get(5)?,
                    block_number: row.get::<_, Option<i64>>(6)?.map(|n| n as u64),
                    block_timestamp: row.get(7)?,
                })
            })
            .map_err(|e| eyre::eyre!("Failed to query events: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| eyre::eyre!("Failed to collect events: {}", e))?;

        debug!("Retrieved {} events for validator {}", events.len(), address);
        Ok(events)
    }

    /// Updates sync state for tracking event processing progress
    pub fn update_sync_state(&self, contract_address: &Address, event_name: &str, block_number: u64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = contract_address.to_checksum(None);

        conn.execute(
            "INSERT INTO sync_state (contract_address, event_name, last_synced_block, last_synced_timestamp, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(contract_address) DO UPDATE SET
             last_synced_block = excluded.last_synced_block,
             last_synced_timestamp = CURRENT_TIMESTAMP,
             updated_at = CURRENT_TIMESTAMP",
            params![&address_str, event_name, block_number as i64],
        ).map_err(|e| eyre::eyre!("Failed to update sync state: {}", e))?;

        debug!("Sync state updated: contract={}, block={}", contract_address, block_number);
        Ok(())
    }

    /// Retrieves last synced block number
    pub fn get_last_synced_block(&self, contract_address: &Address) -> Result<u64> {
        let conn = self.conn.lock().map_err(|e| eyre::eyre!("Failed to lock database: {}", e))?;

        let address_str = contract_address.to_checksum(None);

        let block_number = conn
            .query_row(
                "SELECT COALESCE(last_synced_block, 0) FROM sync_state WHERE contract_address = ?",
                params![&address_str],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| eyre::eyre!("Failed to query sync state: {}", e))?
            .unwrap_or(0);

        Ok(block_number as u64)
    }
}