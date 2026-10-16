use soroban_sdk::{contracttype, Address};

/// Persistent storage keys for the Clips contract.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// The contract administrator.
    Admin,
    /// Authorized royalty manager: DataKey::RoyaltyManager(addr)
    RoyaltyManager(Address),
    /// Royalty configuration for a specific token: DataKey::Royalty(token_id)
    Royalty(u64),
    /// Owner of a specific token: DataKey::Owner(token_id)
    Owner(u64),
    /// Per-token approval:  DataKey::Approval(token_id)
    /// Value is the approved Address.
    Approval(u64),
    /// Operator approval for all tokens owned by `owner`:
    /// DataKey::OperatorApproval(owner, operator)
    OperatorApproval(Address, Address),
    /// Total supply counter.
    TotalSupply,
}
