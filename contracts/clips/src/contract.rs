use crate::approval::{
    approve, get_approval, get_owner, is_operator_approved, revoke_approval,
    revoke_operator_approval, set_approval_for_all, write_owner,
};
use crate::error::ClipsError;
use crate::royalty::{
    add_royalty_manager, freeze_royalty, is_royalty_manager, read_admin, read_royalty,
    remove_royalty_manager, set_royalty, write_admin, RoyaltyConfig,
};
use crate::storage::DataKey;
use soroban_sdk::{contract, contractimpl, Address, Env};

pub trait ClipsContractTrait {
    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// Initialize the contract.  Must be called once before anything else.
    fn initialize(e: Env, admin: Address) -> Result<(), ClipsError>;

    // ── NFT basics ─────────────────────────────────────────────────────────

    /// Mint a new clip NFT to `to`.  Only the admin may mint.
    fn mint(e: Env, caller: Address, to: Address) -> Result<u64, ClipsError>;

    /// Return the owner of `token_id`.
    fn owner_of(e: Env, token_id: u64) -> Result<Address, ClipsError>;

    // ── Royalty management (Issue 1) ───────────────────────────────────────

    /// Add `manager` to the royalty-manager allowlist.  Admin only.
    fn add_royalty_manager(e: Env, caller: Address, manager: Address) -> Result<(), ClipsError>;

    /// Remove `manager` from the royalty-manager allowlist.  Admin only.
    fn remove_royalty_manager(e: Env, caller: Address, manager: Address) -> Result<(), ClipsError>;

    /// Return whether `addr` is a permitted royalty manager.
    fn is_royalty_manager(e: Env, addr: Address) -> bool;

    /// Create or update the royalty configuration for `token_id`.
    /// Caller must be admin or a registered royalty manager.
    fn set_royalty(
        e: Env,
        caller: Address,
        token_id: u64,
        recipient: Address,
        rate_bps: u32,
    ) -> Result<(), ClipsError>;

    /// Freeze the royalty configuration for `token_id` (irreversible).
    /// Caller must be admin or a registered royalty manager.
    fn freeze_royalty(e: Env, caller: Address, token_id: u64) -> Result<(), ClipsError>;

    /// Return the royalty recipient for `token_id`, or a zero value if unset.
    /// We expose rate and frozen status via a separate call to avoid Option.
    fn royalty_recipient(e: Env, token_id: u64) -> Result<Address, ClipsError>;

    /// Return the royalty rate in bps for `token_id`.
    fn royalty_rate_bps(e: Env, token_id: u64) -> Result<u32, ClipsError>;

    /// Return whether the royalty config for `token_id` is frozen.
    fn royalty_frozen(e: Env, token_id: u64) -> Result<bool, ClipsError>;

    // ── Approval management (Issue 2) ──────────────────────────────────────

    /// Grant per-token approval on `token_id` to `approved`.
    /// Caller must be the owner or an approved operator.
    fn approve(e: Env, caller: Address, token_id: u64, approved: Address)
        -> Result<(), ClipsError>;

    /// Revoke per-token approval on `token_id`.
    /// Caller must be the owner or an approved operator.
    fn revoke_approval(e: Env, caller: Address, token_id: u64) -> Result<(), ClipsError>;

    /// Return whether `token_id` has an active per-token approval.
    fn has_approval(e: Env, token_id: u64) -> bool;

    /// Return the approved address for `token_id`, panics if none.
    fn approved_for(e: Env, token_id: u64) -> Result<Address, ClipsError>;

    /// Set (or unset) an operator approval for all of `caller`'s tokens.
    fn set_approval_for_all(
        e: Env,
        caller: Address,
        operator: Address,
        approved: bool,
    ) -> Result<(), ClipsError>;

    /// Revoke operator approval for all of `caller`'s tokens.
    fn revoke_operator_approval(
        e: Env,
        caller: Address,
        operator: Address,
    ) -> Result<(), ClipsError>;

    /// Return whether `operator` is approved for all of `owner`'s tokens.
    fn is_approved_for_all(e: Env, owner: Address, operator: Address) -> bool;
}

#[contract]
pub struct ClipsContract;

#[contractimpl]
impl ClipsContractTrait for ClipsContract {
    fn initialize(e: Env, admin: Address) -> Result<(), ClipsError> {
        if e.storage().instance().has(&DataKey::Admin) {
            return Err(ClipsError::AlreadyInitialized);
        }
        admin.require_auth();
        write_admin(&e, &admin);
        e.storage().instance().set(&DataKey::TotalSupply, &0u64);
        Ok(())
    }

    fn mint(e: Env, caller: Address, to: Address) -> Result<u64, ClipsError> {
        caller.require_auth();
        let admin = read_admin(&e);
        if caller != admin {
            return Err(ClipsError::Unauthorized);
        }
        let supply: u64 = e
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        let token_id = supply;
        write_owner(&e, token_id, &to);
        e.storage()
            .instance()
            .set(&DataKey::TotalSupply, &(supply + 1));
        Ok(token_id)
    }

    fn owner_of(e: Env, token_id: u64) -> Result<Address, ClipsError> {
        get_owner(&e, token_id).ok_or(ClipsError::TokenNotFound)
    }

    // ── Royalty management ────────────────────────────────────────────────

    fn add_royalty_manager(e: Env, caller: Address, manager: Address) -> Result<(), ClipsError> {
        add_royalty_manager(&e, &caller, &manager)
    }

    fn remove_royalty_manager(
        e: Env,
        caller: Address,
        manager: Address,
    ) -> Result<(), ClipsError> {
        remove_royalty_manager(&e, &caller, &manager)
    }

    fn is_royalty_manager(e: Env, addr: Address) -> bool {
        is_royalty_manager(&e, &addr)
    }

    fn set_royalty(
        e: Env,
        caller: Address,
        token_id: u64,
        recipient: Address,
        rate_bps: u32,
    ) -> Result<(), ClipsError> {
        set_royalty(&e, &caller, token_id, recipient, rate_bps)
    }

    fn freeze_royalty(e: Env, caller: Address, token_id: u64) -> Result<(), ClipsError> {
        freeze_royalty(&e, &caller, token_id)
    }

    fn royalty_recipient(e: Env, token_id: u64) -> Result<Address, ClipsError> {
        read_royalty(&e, token_id)
            .ok_or(ClipsError::TokenNotFound)
            .map(|c| c.recipient)
    }

    fn royalty_rate_bps(e: Env, token_id: u64) -> Result<u32, ClipsError> {
        read_royalty(&e, token_id)
            .ok_or(ClipsError::TokenNotFound)
            .map(|c| c.rate_bps)
    }

    fn royalty_frozen(e: Env, token_id: u64) -> Result<bool, ClipsError> {
        read_royalty(&e, token_id)
            .ok_or(ClipsError::TokenNotFound)
            .map(|c| c.frozen)
    }

    // ── Approval management ───────────────────────────────────────────────

    fn approve(
        e: Env,
        caller: Address,
        token_id: u64,
        approved: Address,
    ) -> Result<(), ClipsError> {
        approve(&e, &caller, token_id, &approved)
    }

    fn revoke_approval(e: Env, caller: Address, token_id: u64) -> Result<(), ClipsError> {
        revoke_approval(&e, &caller, token_id)
    }

    fn has_approval(e: Env, token_id: u64) -> bool {
        get_approval(&e, token_id).is_some()
    }

    fn approved_for(e: Env, token_id: u64) -> Result<Address, ClipsError> {
        get_approval(&e, token_id).ok_or(ClipsError::ApprovalNotFound)
    }

    fn set_approval_for_all(
        e: Env,
        caller: Address,
        operator: Address,
        approved: bool,
    ) -> Result<(), ClipsError> {
        let owner = caller.clone();
        set_approval_for_all(&e, &caller, &owner, &operator, approved)
    }

    fn revoke_operator_approval(
        e: Env,
        caller: Address,
        operator: Address,
    ) -> Result<(), ClipsError> {
        let owner = caller.clone();
        revoke_operator_approval(&e, &caller, &owner, &operator)
    }

    fn is_approved_for_all(e: Env, owner: Address, operator: Address) -> bool {
        is_operator_approved(&e, &owner, &operator)
    }
}
