-- 0gchain Indexer SQLite Database Schema

-- Validators table: stores validator information
CREATE TABLE IF NOT EXISTS validators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey TEXT NOT NULL UNIQUE CHECK (LENGTH(pubkey) = 66),
    address TEXT NOT NULL UNIQUE CHECK (LENGTH(address) = 42),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index on address for faster lookups
CREATE INDEX IF NOT EXISTS idx_validators_address ON validators(address);
CREATE INDEX IF NOT EXISTS idx_validators_pubkey ON validators(pubkey);

-- Delegators table: stores delegator information and aggregated delegation amounts
CREATE TABLE IF NOT EXISTS delegators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL UNIQUE CHECK (LENGTH(address) = 42),
    total_delegated TEXT NOT NULL DEFAULT '0',  -- Store as TEXT for BigNumber representation
    total_undelegated TEXT NOT NULL DEFAULT '0',  -- Store as TEXT for BigNumber representation
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index on address for faster lookups
CREATE INDEX IF NOT EXISTS idx_delegators_address ON delegators(address);

-- Validator operator table: tracks commission/tip fee withdrawals by a validator's
-- owner/operator address. Kept separate from `delegators` because these withdrawals
-- are not undelegations and shouldn't be aggregated into total_undelegated.
CREATE TABLE IF NOT EXISTS validator_operator (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    validator_address TEXT NOT NULL CHECK (LENGTH(validator_address) = 42),
    owner_address TEXT NOT NULL CHECK (LENGTH(owner_address) = 42),
    total_commission_withdrawn TEXT NOT NULL DEFAULT '0',  -- Store as TEXT for BigNumber representation
    total_tip_fee_withdrawn TEXT NOT NULL DEFAULT '0',  -- Store as TEXT for BigNumber representation
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(validator_address, owner_address)
);

-- Create indexes on validator/owner for faster lookups
CREATE INDEX IF NOT EXISTS idx_validator_operator_validator ON validator_operator(validator_address);
CREATE INDEX IF NOT EXISTS idx_validator_operator_owner ON validator_operator(owner_address);

-- Events table: stores all delegation and undelegation events
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    delegator_address TEXT NOT NULL CHECK (LENGTH(delegator_address) = 42),
    validator_address TEXT NOT NULL CHECK (LENGTH(validator_address) = 42),
    event_type INTEGER NOT NULL,  -- 0: Delegate, 1: Undelegate, 2: WithdrawCommission, 3: WithdrawTipFee
    amount TEXT NOT NULL,  -- Store as TEXT for BigNumber representation
    shares TEXT NOT NULL,  -- Store as TEXT for BigNumber representation
    transaction_hash TEXT,
    block_number INTEGER,
    block_timestamp TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for faster event lookups
CREATE INDEX IF NOT EXISTS idx_events_delegator ON events(delegator_address);
CREATE INDEX IF NOT EXISTS idx_events_validator ON events(validator_address);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_block ON events(block_number);

-- Create tracking table for monitoring contract event sync progress
CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_address TEXT NOT NULL UNIQUE CHECK (LENGTH(contract_address) = 42),
    event_name TEXT NOT NULL,
    last_synced_block INTEGER DEFAULT 0,
    last_synced_timestamp TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sync_state_contract ON sync_state(contract_address);
