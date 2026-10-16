//! Royalty authorization module.
//!
//! Implements Issue: [Authorization] Restrict Royalty Configuration Changes
//! - Define permitted royalty managers stored on-chain.
//! - Validate authorization before any royalty create/update/freeze.
//! - Prevent unauthorized recipient or rate modifications.

use crate::error::ClipsError;
use crate::storage::DataKey;
use soroban_sdk::{contracttype, Address, Env};

/// On-chain royalty configuration for a single token.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct RoyaltyConfig {
    /// The address that receives royalty payments.
    pub recipient: Address,
    /// Royalty rate in basis points (0–10_000). 100 bp = 1%.
    pub rate_bps: u32,
    /// When `true` the configuration can no longer be changed.
    pub frozen: bool,
}

// ─── Storage helpers ────────────────────────────────────────────────────────

/// Return the stored administrator or panic with `NotInitialized`.
pub fn read_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"))
}

/// Store the administrator.
pub fn write_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

/// Return `true` if `addr` is listed as a royalty manager.
pub fn is_royalty_manager(e: &Env, addr: &Address) -> bool {
    e.storage()
        .persistent()
        .get::<_, bool>(&DataKey::RoyaltyManager(addr.clone()))
        .unwrap_or(false)
}

/// Add `addr` to the royalty-manager allowlist.  Requires admin auth.
///
/// # Errors
/// - [`ClipsError::Unauthorized`] if the caller is not the admin.
pub fn add_royalty_manager(e: &Env, caller: &Address, manager: &Address) -> Result<(), ClipsError> {
    caller.require_auth();
    let admin = read_admin(e);
    if *caller != admin {
        return Err(ClipsError::Unauthorized);
    }
    e.storage()
        .persistent()
        .set(&DataKey::RoyaltyManager(manager.clone()), &true);
    Ok(())
}

/// Remove `addr` from the royalty-manager allowlist.  Requires admin auth.
///
/// # Errors
/// - [`ClipsError::Unauthorized`] if the caller is not the admin.
pub fn remove_royalty_manager(
    e: &Env,
    caller: &Address,
    manager: &Address,
) -> Result<(), ClipsError> {
    caller.require_auth();
    let admin = read_admin(e);
    if *caller != admin {
        return Err(ClipsError::Unauthorized);
    }
    e.storage()
        .persistent()
        .remove(&DataKey::RoyaltyManager(manager.clone()));
    Ok(())
}

/// Read the royalty config for `token_id`.  Returns `None` if not set.
pub fn read_royalty(e: &Env, token_id: u64) -> Option<RoyaltyConfig> {
    e.storage()
        .persistent()
        .get::<_, RoyaltyConfig>(&DataKey::Royalty(token_id))
}

/// Create or update the royalty configuration for `token_id`.
///
/// Authorization rules:
/// 1. `caller.require_auth()` is called unconditionally (Soroban auth pipeline).
/// 2. `caller` must be either the contract admin **or** a registered royalty manager.
/// 3. The config must not be frozen.
/// 4. `rate_bps` must be in [0, 10_000].
///
/// # Errors
/// - [`ClipsError::Unauthorized`]   – caller lacks permission.
/// - [`ClipsError::InvalidRoyaltyRate`] – rate is > 10_000 bp.
/// - [`ClipsError::Unauthorized`]   – config is frozen (reuses same variant for simplicity).
pub fn set_royalty(
    e: &Env,
    caller: &Address,
    token_id: u64,
    recipient: Address,
    rate_bps: u32,
) -> Result<(), ClipsError> {
    // Always require auth from the caller (Soroban authorization pipeline).
    caller.require_auth();

    // Validate rate range.
    if rate_bps > 10_000 {
        return Err(ClipsError::InvalidRoyaltyRate);
    }

    // Check existing config for frozen flag before authorizing mutation.
    if let Some(existing) = read_royalty(e, token_id) {
        if existing.frozen {
            return Err(ClipsError::Unauthorized);
        }
    }

    // Enforce the manager allowlist: caller must be admin OR a registered manager.
    let admin = read_admin(e);
    if *caller != admin && !is_royalty_manager(e, caller) {
        return Err(ClipsError::Unauthorized);
    }

    let config = RoyaltyConfig {
        recipient,
        rate_bps,
        frozen: false,
    };
    e.storage()
        .persistent()
        .set(&DataKey::Royalty(token_id), &config);
    Ok(())
}

/// Freeze the royalty configuration for `token_id`, preventing future changes.
///
/// Requires the same authorization as `set_royalty`.
///
/// # Errors
/// - [`ClipsError::Unauthorized`] – caller lacks permission.
/// - [`ClipsError::TokenNotFound`] – no royalty config exists for this token.
pub fn freeze_royalty(e: &Env, caller: &Address, token_id: u64) -> Result<(), ClipsError> {
    caller.require_auth();

    let admin = read_admin(e);
    if *caller != admin && !is_royalty_manager(e, caller) {
        return Err(ClipsError::Unauthorized);
    }

    let mut config = read_royalty(e, token_id).ok_or(ClipsError::TokenNotFound)?;
    config.frozen = true;
    e.storage()
        .persistent()
        .set(&DataKey::Royalty(token_id), &config);
    Ok(())
}
