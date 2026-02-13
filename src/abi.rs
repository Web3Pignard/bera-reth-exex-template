// ABI definitions for 0gchain staking contracts
// Defines event signatures and log topics for event filtering

use alloy_primitives::Address;
use lazy_static::lazy_static;

// ValidatorStaking contract address
pub const VALIDATOR_STAKING_ADDRESS: &str = "0xea224dBB52F57752044c0C86aD50930091F561B9";

// Event signatures (keccak256 hashes)
lazy_static! {
    /// ValidatorCreated(bytes,address)
    pub static ref VALIDATOR_CREATED_TOPIC: [u8; 32] = {
        let hash = alloy_primitives::keccak256(b"ValidatorCreated(bytes,address)");
        hash.0
    };
    
    /// Delegate(address,address,uint256,uint256)
    pub static ref DELEGATE_TOPIC: [u8; 32] = {
        let hash = alloy_primitives::keccak256(b"Delegate(address,address,uint256,uint256)");
        hash.0
    };
    
    /// Undelegate(address,address,uint256,uint256)
    pub static ref UNDELEGATE_TOPIC: [u8; 32] = {
        let hash = alloy_primitives::keccak256(b"Undelegate(address,address,uint256,uint256)");
        hash.0
    };
    
    /// Redelegate(address,address,uint256,uint256)
    pub static ref REDELEGATE_TOPIC: [u8; 32] = {
        let hash = alloy_primitives::keccak256(b"Redelegate(address,address,uint256,uint256)");
        hash.0
    };
}

/// Returns the ValidatorStaking contract address
pub fn get_validator_staking_address() -> Address {
    Address::parse_checksummed(VALIDATOR_STAKING_ADDRESS, None)
        .expect("Invalid validator staking address")
}

/// Checks if a log topic matches ValidatorCreated event
pub fn is_validator_created_event(topic: &[u8; 32]) -> bool {
    topic == &VALIDATOR_CREATED_TOPIC[..]
}

/// Checks if a log topic matches Delegate event
pub fn is_delegate_event(topic: &[u8; 32]) -> bool {
    topic == &DELEGATE_TOPIC[..]
}

/// Checks if a log topic matches Undelegate event
pub fn is_undelegate_event(topic: &[u8; 32]) -> bool {
    topic == &UNDELEGATE_TOPIC[..]
}

/// Checks if a log topic matches Redelegate event
pub fn is_redelegate_event(topic: &[u8; 32]) -> bool {
    topic == &REDELEGATE_TOPIC[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_generation() {
        // Verify topics are correctly generated
        assert_eq!(VALIDATOR_CREATED_TOPIC.len(), 32);
        assert_eq!(DELEGATE_TOPIC.len(), 32);
        assert_eq!(UNDELEGATE_TOPIC.len(), 32);
        assert_eq!(REDELEGATE_TOPIC.len(), 32);
    }

    #[test]
    fn test_address_parsing() {
        let addr = get_validator_staking_address();
        assert_eq!(addr.to_string().to_lowercase(), VALIDATOR_STAKING_ADDRESS.to_lowercase());
    }
}