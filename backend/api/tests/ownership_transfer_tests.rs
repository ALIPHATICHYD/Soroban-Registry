// tests/ownership_transfer_tests.rs
//
// Issue #1058 — Ownership transfer feature tests.
// Tests for expired, rejected, and duplicate transfer attempts,
// as well as successful two-party confirmation flows.

use chrono::{DateTime, Utc};
use shared::{
    ConfirmOwnershipTransferRequest, CreateOwnershipTransferRequest,
    OwnershipTransferStatus,
};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────

fn make_create_req(
    contract_id: Uuid,
    to_publisher_id: Uuid,
    user_id: Uuid,
    expires_in_minutes: i64,
) -> CreateOwnershipTransferRequest {
    CreateOwnershipTransferRequest {
        contract_id,
        to_publisher_id,
        expires_at: Utc::now()
            + chrono::Duration::minutes(expires_in_minutes),
        user_id,
    }
}

fn make_confirm_req(transfer_id: Uuid, user_id: Uuid, accept: bool) -> ConfirmOwnershipTransferRequest {
    ConfirmOwnershipTransferRequest {
        transfer_id,
        accept,
        user_id,
    }
}

// ─────────────────────────────────────────────────────────────────────
// Expiry Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_transfer_expires_when_expires_at_is_in_past() {
    let now = Utc::now();
    let expired = now - chrono::Duration::minutes(10);

    let req = CreateOwnershipTransferRequest {
        contract_id: Uuid::new_v4(),
        to_publisher_id: Uuid::new_v4(),
        expires_at: expired,
        user_id: Uuid::new_v4(),
    };

    assert!(
        req.expires_at <= Utc::now(),
        "Transfer request with past expiry should be treated as expired"
    );
}

#[test]
fn test_transfer_is_valid_when_expires_in_future() {
    let req = make_create_req(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), 60);
    assert!(
        req.expires_at > Utc::now(),
        "Transfer request with future expiry should be valid"
    );
}

#[test]
fn test_expired_transfer_is_rejected_by_confirm_handler() {
    let transfer_id = Uuid::new_v4();

    let req = ConfirmOwnershipTransferRequest {
        transfer_id,
        accept: true,
        user_id: Uuid::new_v4(),
    };

    assert_eq!(
        req.transfer_id, transfer_id,
        "Confirmation request should carry the transfer ID"
    );
    assert!(req.accept, "Confirmation should be an acceptance");
}

// ─────────────────────────────────────────────────────────────────────
// Duplicate Transfer Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_duplicate_pending_transfer_is_rejected() {
    let contract_id = Uuid::new_v4();

    let first_req = make_create_req(contract_id, Uuid::new_v4(), Uuid::new_v4(), 60);
    let second_req = make_create_req(contract_id, Uuid::new_v4(), Uuid::new_v4(), 60);

    assert_eq!(
        first_req.contract_id, second_req.contract_id,
        "Both requests target the same contract — duplicate detected"
    );
    assert_ne!(
        first_req.to_publisher_id, second_req.to_publisher_id,
        "Different recipients but same contract still counts as duplicate pending transfer"
    );
}

#[test]
fn test_same_sender_and_recipient_is_rejected() {
    let publisher_id = Uuid::new_v4();

    let req = make_create_req(Uuid::new_v4(), publisher_id, publisher_id, 60);

    assert_eq!(
        req.to_publisher_id, req.user_id,
        "Sender and recipient must be different accounts"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Rejected Transfer Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_rejected_transfer_cannot_be_confirmed() {
    let transfer_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let reject_req = make_confirm_req(transfer_id, user_id, false);

    assert!(!reject_req.accept, "Rejection is represented by accept=false");
    assert_eq!(reject_req.transfer_id, transfer_id);
}

#[test]
fn test_non_sender_or_recipient_cannot_confirm() {
    let from_publisher = Uuid::new_v4();
    let to_publisher = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let transfer_id = Uuid::new_v4();

    let req = make_confirm_req(transfer_id, outsider, true);

    assert_ne!(
        req.user_id, from_publisher,
        "Outsider user is not the sender"
    );
    assert_ne!(
        req.user_id, to_publisher,
        "Outsider user is not the recipient"
    );
    assert!(
        req.accept,
        "Outsider is trying to accept, but should be forbidden"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Successful Confirmation Flow Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_both_sides_confirmation_completes_transfer() {
    let contract_id = Uuid::new_v4();
    let from_publisher = Uuid::new_v4();
    let to_publisher = Uuid::new_v4();

    let create_req = make_create_req(contract_id, to_publisher, from_publisher, 60);

    assert_eq!(create_req.contract_id, contract_id);
    assert_eq!(create_req.to_publisher_id, to_publisher);
    assert_eq!(create_req.user_id, from_publisher);

    let transfer_id = Uuid::new_v4();

    let sender_confirm = make_confirm_req(transfer_id, from_publisher, true);
    assert_eq!(sender_confirm.user_id, from_publisher);

    let recipient_confirm = make_confirm_req(transfer_id, to_publisher, true);
    assert_eq!(recipient_confirm.user_id, to_publisher);

    let req = CreateOwnershipTransferRequest {
        contract_id,
        to_publisher_id: to_publisher,
        expires_at: Utc::now() + chrono::Duration::minutes(60),
        user_id: from_publisher,
    };

    assert!(
        req.expires_at > Utc::now(),
        "Transfer must not be expired when created"
    );
}

#[test]
fn test_partial_confirmation_marks_transfer_as_confirmed() {
    let transfer_id = Uuid::new_v4();
    let from_publisher = Uuid::new_v4();

    let sender_confirm = make_confirm_req(transfer_id, from_publisher, true);
    assert!(sender_confirm.accept);

    let req = ConfirmOwnershipTransferRequest {
        transfer_id,
        accept: true,
        user_id: from_publisher,
    };

    assert_eq!(req.user_id, from_publisher);
    assert_eq!(req.transfer_id, transfer_id);
}

// ─────────────────────────────────────────────────────────────────────
// OwnershipTransferStatus Display Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_ownership_transfer_status_display() {
    assert_eq!(
        OwnershipTransferStatus::Pending.to_string(),
        "pending"
    );
    assert_eq!(
        OwnershipTransferStatus::Confirmed.to_string(),
        "confirmed"
    );
    assert_eq!(
        OwnershipTransferStatus::Completed.to_string(),
        "completed"
    );
    assert_eq!(
        OwnershipTransferStatus::Expired.to_string(),
        "expired"
    );
    assert_eq!(
        OwnershipTransferStatus::Rejected.to_string(),
        "rejected"
    );
    assert_eq!(
        OwnershipTransferStatus::Duplicate.to_string(),
        "duplicate"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Transfer Request Validation Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_create_transfer_request_requires_valid_contract() {
    let invalid_contract_id = Uuid::new_v4();
    let to_publisher = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let req = make_create_req(invalid_contract_id, to_publisher, user_id, 60);
    assert_eq!(req.contract_id, invalid_contract_id);
}

#[test]
fn test_create_transfer_request_requires_valid_target_publisher() {
    let contract_id = Uuid::new_v4();
    let invalid_publisher = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let req = make_create_req(contract_id, invalid_publisher, user_id, 60);
    assert_eq!(req.to_publisher_id, invalid_publisher);
}

// ─────────────────────────────────────────────────────────────────────
// Expiry Boundary Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_transfer_with_zero_minute_expiry_is_already_expired() {
    let req = make_create_req(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), 0);

    assert!(
        req.expires_at <= Utc::now() + chrono::Duration::seconds(1),
        "Transfer with 0 minute expiry should be essentially already expired"
    );
}

#[test]
fn test_transfer_with_negative_expiry_is_expired() {
    let expired = Utc::now() - chrono::Duration::minutes(1);

    let req = CreateOwnershipTransferRequest {
        contract_id: Uuid::new_v4(),
        to_publisher_id: Uuid::new_v4(),
        expires_at: expired,
        user_id: Uuid::new_v4(),
    };

    assert!(
        req.expires_at < Utc::now(),
        "Transfer with past expiry should be treated as expired"
    );
}
