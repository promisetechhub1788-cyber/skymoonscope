#![cfg(test)]

use crate::error::ClipsError;
use crate::contract::{ClipsContract, ClipsContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (Address, ClipsContractClient<'_>) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let contract_id = env.register(ClipsContract, ());
    let client = ClipsContractClient::new(env, &contract_id);
    client.initialize(&admin).unwrap();
    (admin, client)
}

fn mint_token(client: &ClipsContractClient<'_>, admin: &Address, owner: &Address) -> u64 {
    client.mint(admin, owner).unwrap()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Issue 1 – [Authorization] Restrict Royalty Configuration Changes
// ═══════════════════════════════════════════════════════════════════════════════

// ── AC-1: Define permitted royalty managers ───────────────────────────────────

#[test]
fn admin_can_add_royalty_manager() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let manager = Address::generate(&env);

    assert!(!client.is_royalty_manager(&manager));
    client.add_royalty_manager(&admin, &manager).unwrap();
    assert!(client.is_royalty_manager(&manager));
}

#[test]
fn admin_can_remove_royalty_manager() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let manager = Address::generate(&env);

    client.add_royalty_manager(&admin, &manager).unwrap();
    assert!(client.is_royalty_manager(&manager));

    client.remove_royalty_manager(&admin, &manager).unwrap();
    assert!(!client.is_royalty_manager(&manager));
}

#[test]
fn non_admin_cannot_add_royalty_manager() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    let stranger = Address::generate(&env);
    let target = Address::generate(&env);

    let err = client
        .try_add_royalty_manager(&stranger, &target)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);
}

#[test]
fn non_admin_cannot_remove_royalty_manager() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let manager = Address::generate(&env);
    let stranger = Address::generate(&env);

    client.add_royalty_manager(&admin, &manager).unwrap();

    let err = client
        .try_remove_royalty_manager(&stranger, &manager)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);

    // Manager is still registered after the failed attempt.
    assert!(client.is_royalty_manager(&manager));
}

// ── AC-2: Validate authorization before changes ───────────────────────────────

#[test]
fn admin_can_set_royalty() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client
        .set_royalty(&admin, &token_id, &recipient, &500u32)
        .unwrap();

    assert_eq!(client.royalty_recipient(&token_id).unwrap(), recipient);
    assert_eq!(client.royalty_rate_bps(&token_id).unwrap(), 500u32);
    assert!(!client.royalty_frozen(&token_id).unwrap());
}

#[test]
fn registered_manager_can_set_royalty() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let manager = Address::generate(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.add_royalty_manager(&admin, &manager).unwrap();
    let token_id = mint_token(&client, &admin, &owner);

    client
        .set_royalty(&manager, &token_id, &recipient, &250u32)
        .unwrap();

    assert_eq!(client.royalty_rate_bps(&token_id).unwrap(), 250u32);
}

#[test]
fn unauthorized_caller_cannot_set_royalty() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let stranger = Address::generate(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);

    let err = client
        .try_set_royalty(&stranger, &token_id, &recipient, &100u32)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);
}

// ── AC-3: Prevent unauthorized recipient or rate modifications ────────────────

#[test]
fn stranger_cannot_modify_royalty_config() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let original_recipient = Address::generate(&env);
    let new_recipient = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client
        .set_royalty(&admin, &token_id, &original_recipient, &500u32)
        .unwrap();

    let stranger = Address::generate(&env);
    let _ = client.try_set_royalty(&stranger, &token_id, &new_recipient, &9999u32);

    // Original config must be intact.
    assert_eq!(
        client.royalty_recipient(&token_id).unwrap(),
        original_recipient
    );
    assert_eq!(client.royalty_rate_bps(&token_id).unwrap(), 500u32);
}

#[test]
fn invalid_royalty_rate_rejected() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);

    let err = client
        .try_set_royalty(&admin, &token_id, &recipient, &10_001u32)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::InvalidRoyaltyRate);
}

#[test]
fn frozen_royalty_cannot_be_modified() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let new_recipient = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client
        .set_royalty(&admin, &token_id, &recipient, &300u32)
        .unwrap();
    client.freeze_royalty(&admin, &token_id).unwrap();

    // Even admin cannot change a frozen config.
    let err = client
        .try_set_royalty(&admin, &token_id, &new_recipient, &999u32)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);

    // Config is unchanged and still frozen.
    assert_eq!(client.royalty_recipient(&token_id).unwrap(), recipient);
    assert_eq!(client.royalty_rate_bps(&token_id).unwrap(), 300u32);
    assert!(client.royalty_frozen(&token_id).unwrap());
}

#[test]
fn unauthorized_caller_cannot_freeze_royalty() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let stranger = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client
        .set_royalty(&admin, &token_id, &recipient, &200u32)
        .unwrap();

    let err = client.try_freeze_royalty(&stranger, &token_id).unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);

    // Config is still unfrozen.
    assert!(!client.royalty_frozen(&token_id).unwrap());
}

#[test]
fn removed_manager_cannot_set_royalty() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let manager = Address::generate(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.add_royalty_manager(&admin, &manager).unwrap();
    let token_id = mint_token(&client, &admin, &owner);

    // Revoke manager; they can no longer set royalties.
    client.remove_royalty_manager(&admin, &manager).unwrap();

    let err = client
        .try_set_royalty(&manager, &token_id, &recipient, &100u32)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Issue 2 – [Authorization] Prevent Unauthorized Approval Revocation
// ═══════════════════════════════════════════════════════════════════════════════

// ── AC-1: Verify the caller's authority over the approval ─────────────────────

#[test]
fn token_owner_can_revoke_per_token_approval() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client.approve(&owner, &token_id, &spender).unwrap();

    assert!(client.has_approval(&token_id));
    client.revoke_approval(&owner, &token_id).unwrap();
    assert!(!client.has_approval(&token_id));
}

#[test]
fn operator_can_revoke_per_token_approval() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let operator = Address::generate(&env);
    let spender = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client
        .set_approval_for_all(&owner, &operator, &true)
        .unwrap();
    client.approve(&owner, &token_id, &spender).unwrap();

    // Operator is authorized to revoke.
    client.revoke_approval(&operator, &token_id).unwrap();
    assert!(!client.has_approval(&token_id));
}

// ── AC-2: Reject unauthorized revocation requests ─────────────────────────────

#[test]
fn stranger_cannot_revoke_per_token_approval() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let stranger = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client.approve(&owner, &token_id, &spender).unwrap();

    let err = client
        .try_revoke_approval(&stranger, &token_id)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);
}

#[test]
fn approved_spender_cannot_revoke_own_approval() {
    // The spender themselves must NOT be able to revoke their own approval.
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client.approve(&owner, &token_id, &spender).unwrap();

    let err = client
        .try_revoke_approval(&spender, &token_id)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);
}

#[test]
fn revoke_nonexistent_approval_returns_not_found() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);

    // No approval was set.
    let err = client
        .try_revoke_approval(&owner, &token_id)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::ApprovalNotFound);
}

// ── AC-3: Preserve valid approvals after failed requests ──────────────────────

#[test]
fn valid_approval_preserved_after_unauthorized_revoke_attempt() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let stranger = Address::generate(&env);

    let token_id = mint_token(&client, &admin, &owner);
    client.approve(&owner, &token_id, &spender).unwrap();

    // Unauthorized attempt; must not touch the approval.
    let _ = client.try_revoke_approval(&stranger, &token_id);

    assert!(client.has_approval(&token_id));
    assert_eq!(client.approved_for(&token_id).unwrap(), spender);
}

// ── Operator approval revocation ──────────────────────────────────────────────

#[test]
fn owner_can_revoke_operator_approval() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let operator = Address::generate(&env);

    let _token_id = mint_token(&client, &admin, &owner);
    client
        .set_approval_for_all(&owner, &operator, &true)
        .unwrap();
    assert!(client.is_approved_for_all(&owner, &operator));

    client.revoke_operator_approval(&owner, &operator).unwrap();
    assert!(!client.is_approved_for_all(&owner, &operator));
}

#[test]
fn stranger_cannot_revoke_operator_approval() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let operator = Address::generate(&env);
    let stranger = Address::generate(&env);

    let _token_id = mint_token(&client, &admin, &owner);
    client
        .set_approval_for_all(&owner, &operator, &true)
        .unwrap();

    let err = client
        .try_revoke_operator_approval(&stranger, &operator)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::Unauthorized);

    // Operator approval must still be active.
    assert!(client.is_approved_for_all(&owner, &operator));
}

#[test]
fn revoke_nonexistent_operator_approval_returns_not_found() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let operator = Address::generate(&env);

    let _token_id = mint_token(&client, &admin, &owner);

    let err = client
        .try_revoke_operator_approval(&owner, &operator)
        .unwrap_err();
    assert_eq!(err.unwrap(), ClipsError::ApprovalNotFound);
}

#[test]
fn operator_approval_preserved_after_unauthorized_revoke_attempt() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let owner = Address::generate(&env);
    let operator = Address::generate(&env);
    let stranger = Address::generate(&env);

    let _token_id = mint_token(&client, &admin, &owner);
    client
        .set_approval_for_all(&owner, &operator, &true)
        .unwrap();

    // Unauthorized attempt.
    let _ = client.try_revoke_operator_approval(&stranger, &operator);

    // Approval must still be active.
    assert!(client.is_approved_for_all(&owner, &operator));
}
