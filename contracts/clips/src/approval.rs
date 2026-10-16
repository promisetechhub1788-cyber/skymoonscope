//! Approval revocation authorization module.
//!
//! Implements Issue: [Authorization] Prevent Unauthorized Approval Revocation
//! - Verify the caller's authority over the approval before revoking it.
//! - Reject unauthorized revocation requests (return an error, not a panic).
//! - Preserve valid approvals after a failed revocation attempt.

use crate::error::ClipsError;
use crate::storage::DataKey;
use soroban_sdk::{Address, Env};

// ─── Per-token (ERC-721-style) approvals ────────────────────────────────────

/// Return the address approved for `token_id`, if any.
pub fn get_approval(e: &Env, token_id: u64) -> Option<Address> {
    e.storage()
        .persistent()
        .get::<_, Address>(&DataKey::Approval(token_id))
}

/// Read the owner of `token_id`.  Returns `None` if the token does not exist.
pub fn get_owner(e: &Env, token_id: u64) -> Option<Address> {
    e.storage()
        .persistent()
        .get::<_, Address>(&DataKey::Owner(token_id))
}

/// Write the owner of `token_id`.
pub fn write_owner(e: &Env, token_id: u64, owner: &Address) {
    e.storage()
        .persistent()
        .set(&DataKey::Owner(token_id), owner);
}

/// Grant per-token approval to `approved` for `token_id`.
///
/// The `caller` must be the token owner or an operator approved for all tokens
/// owned by the owner.
///
/// # Errors
/// - [`ClipsError::TokenNotFound`] – `token_id` has no owner.
/// - [`ClipsError::Unauthorized`]  – `caller` is not the owner or an operator.
pub fn approve(
    e: &Env,
    caller: &Address,
    token_id: u64,
    approved: &Address,
) -> Result<(), ClipsError> {
    caller.require_auth();

    let owner = get_owner(e, token_id).ok_or(ClipsError::TokenNotFound)?;

    // Caller must be owner or an operator approved for all of owner's tokens.
    if *caller != owner && !is_operator_approved(e, &owner, caller) {
        return Err(ClipsError::Unauthorized);
    }

    e.storage()
        .persistent()
        .set(&DataKey::Approval(token_id), approved);
    Ok(())
}

/// Revoke per-token approval for `token_id`.
///
/// Authorization rules:
/// 1. `caller.require_auth()` ensures a valid Soroban auth context.
/// 2. `caller` must be:
///    - the **token owner**, OR
///    - an **operator** approved for all tokens owned by the token owner.
///
/// If the caller does not meet those criteria the function returns
/// [`ClipsError::Unauthorized`] and the approval is left intact.
///
/// # Errors
/// - [`ClipsError::TokenNotFound`]  – `token_id` has no registered owner.
/// - [`ClipsError::ApprovalNotFound`] – no approval exists for `token_id`.
/// - [`ClipsError::Unauthorized`]   – caller is neither owner nor operator.
pub fn revoke_approval(e: &Env, caller: &Address, token_id: u64) -> Result<(), ClipsError> {
    // 1. Demand a valid Soroban auth context for the caller.
    caller.require_auth();

    // 2. Verify the token exists and resolve its owner.
    let owner = get_owner(e, token_id).ok_or(ClipsError::TokenNotFound)?;

    // 3. Ensure there is actually an approval to revoke.
    if get_approval(e, token_id).is_none() {
        return Err(ClipsError::ApprovalNotFound);
    }

    // 4. Verify the caller's authority:
    //    - token owner, OR
    //    - operator approved for all of owner's tokens.
    //    Any other caller is rejected, and the approval is preserved untouched.
    if *caller != owner && !is_operator_approved(e, &owner, caller) {
        return Err(ClipsError::Unauthorized);
    }

    // 5. Authorized – remove the approval.
    e.storage()
        .persistent()
        .remove(&DataKey::Approval(token_id));
    Ok(())
}

// ─── Operator (approve-for-all) approvals ───────────────────────────────────

/// Return `true` if `operator` is approved to manage all tokens owned by `owner`.
pub fn is_operator_approved(e: &Env, owner: &Address, operator: &Address) -> bool {
    e.storage()
        .persistent()
        .get::<_, bool>(&DataKey::OperatorApproval(
            owner.clone(),
            operator.clone(),
        ))
        .unwrap_or(false)
}

/// Set or revoke an operator approval for all tokens owned by `owner`.
///
/// Only the `owner` may change their own operator approvals.
///
/// # Errors
/// - [`ClipsError::Unauthorized`] – `caller` is not `owner`.
pub fn set_approval_for_all(
    e: &Env,
    caller: &Address,
    owner: &Address,
    operator: &Address,
    approved: bool,
) -> Result<(), ClipsError> {
    caller.require_auth();

    // Only the token owner controls their own operator list.
    if caller != owner {
        return Err(ClipsError::Unauthorized);
    }

    let key = DataKey::OperatorApproval(owner.clone(), operator.clone());
    if approved {
        e.storage().persistent().set(&key, &true);
    } else {
        e.storage().persistent().remove(&key);
    }
    Ok(())
}

/// Revoke an operator approval for all tokens owned by `owner`.
///
/// Authorization rules mirror `set_approval_for_all`:
/// - `caller` must be the `owner`.
///
/// # Errors
/// - [`ClipsError::Unauthorized`] – `caller` is not `owner`.
/// - [`ClipsError::ApprovalNotFound`] – the operator approval does not exist.
pub fn revoke_operator_approval(
    e: &Env,
    caller: &Address,
    owner: &Address,
    operator: &Address,
) -> Result<(), ClipsError> {
    caller.require_auth();

    if caller != owner {
        return Err(ClipsError::Unauthorized);
    }

    let key = DataKey::OperatorApproval(owner.clone(), operator.clone());
    if !e
        .storage()
        .persistent()
        .get::<_, bool>(&key)
        .unwrap_or(false)
    {
        return Err(ClipsError::ApprovalNotFound);
    }

    e.storage().persistent().remove(&key);
    Ok(())
}
