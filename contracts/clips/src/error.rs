use soroban_sdk::contracterror;

/// Contract-level errors for the Clips NFT contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClipsError {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// The caller is not authorized to perform this action.
    Unauthorized = 3,
    /// The token ID does not exist.
    TokenNotFound = 4,
    /// A token with this ID has already been minted.
    AlreadyMinted = 5,
    /// The royalty rate is out of the allowed range (0–10_000 basis points).
    InvalidRoyaltyRate = 6,
    /// The approval being revoked does not exist.
    ApprovalNotFound = 7,
    /// The royalty manager address is not valid (e.g. zero address).
    InvalidManager = 8,
}
