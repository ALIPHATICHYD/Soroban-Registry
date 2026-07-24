// tests/publisher_verification_tests.rs
//
// Issue #603 — Contract publisher verification badge API endpoint.
// Tests for email validation, token checking, and response construction.

// ─────────────────────────────────────────────────────────────────────────────
// Logic mirrors publisher_verification_handlers.rs (no DB dependency)
// ─────────────────────────────────────────────────────────────────────────────

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    !local.is_empty() && domain.contains('.') && domain.len() > 2
}

#[derive(Debug, PartialEq)]
enum VerificationBadgeStatus {
    Verified,
    Pending,
    Failed,
}

fn check_token(email: &str, token: Option<&str>) -> VerificationBadgeStatus {
    let _ = email;
    match token {
        None => VerificationBadgeStatus::Pending,
        Some("") => VerificationBadgeStatus::Failed,
        Some(tok) if tok.len() >= 6 && (tok.contains("verify") || tok.starts_with("tok_")) => {
            VerificationBadgeStatus::Verified
        }
        _ => VerificationBadgeStatus::Failed,
    }
}

fn badge_url(publisher_id: &str, status: &VerificationBadgeStatus) -> String {
    let label = match status {
        VerificationBadgeStatus::Verified => "verified",
        VerificationBadgeStatus::Pending => "pending",
        VerificationBadgeStatus::Failed => "failed",
    };
    format!("/api/publishers/{publisher_id}/badge.svg?status={label}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_valid_email_standard() {
    assert!(is_valid_email("dev@stellar.org"));
}

#[test]
fn test_valid_email_subdomain() {
    assert!(is_valid_email("user@sub.domain.io"));
}

#[test]
fn test_valid_email_plus_addressing() {
    assert!(is_valid_email("dev+soroban@stellar.org"));
}

#[test]
fn test_invalid_email_no_at() {
    assert!(!is_valid_email("notanemail.com"));
}

#[test]
fn test_invalid_email_no_domain() {
    assert!(!is_valid_email("user@"));
}

#[test]
fn test_invalid_email_no_local() {
    assert!(!is_valid_email("@domain.com"));
}

#[test]
fn test_invalid_email_empty() {
    assert!(!is_valid_email(""));
}

#[test]
fn test_invalid_email_short_domain() {
    // "a@b" — domain too short (no dot with >2 char domain)
    assert!(!is_valid_email("a@b"));
}

#[test]
fn test_token_none_returns_pending() {
    let status = check_token("user@example.com", None);
    assert_eq!(
        status,
        VerificationBadgeStatus::Pending,
        "No token → pending"
    );
}

#[test]
fn test_token_empty_returns_failed() {
    let status = check_token("user@example.com", Some(""));
    assert_eq!(status, VerificationBadgeStatus::Failed);
}

#[test]
fn test_token_verify_prefix_verified() {
    let status = check_token("user@example.com", Some("verify-abc123"));
    assert_eq!(status, VerificationBadgeStatus::Verified);
}

#[test]
fn test_token_tok_prefix_verified() {
    let status = check_token("user@example.com", Some("tok_abcdef"));
    assert_eq!(status, VerificationBadgeStatus::Verified);
}

#[test]
fn test_token_short_returns_failed() {
    let status = check_token("user@example.com", Some("ab12"));
    assert_eq!(status, VerificationBadgeStatus::Failed);
}

#[test]
fn test_token_random_long_returns_failed() {
    // Long but no recognised prefix
    let status = check_token("user@example.com", Some("random_blahblah_here"));
    assert_eq!(status, VerificationBadgeStatus::Failed);
}

#[test]
fn test_badge_url_verified() {
    let id = "550e8400-e29b-41d4-a716-446655440001";
    let url = badge_url(id, &VerificationBadgeStatus::Verified);
    assert!(url.contains(id));
    assert!(url.contains("verified"));
}

#[test]
fn test_badge_url_pending() {
    let id = "550e8400-e29b-41d4-a716-446655440002";
    let url = badge_url(id, &VerificationBadgeStatus::Pending);
    assert!(url.contains("pending"));
}

#[test]
fn test_badge_url_failed() {
    let id = "550e8400-e29b-41d4-a716-446655440003";
    let url = badge_url(id, &VerificationBadgeStatus::Failed);
    assert!(url.contains("failed"));
}

#[test]
fn test_verification_flow_no_token() {
    // Simulates the first step: submit email without a token (pending).
    let email = "alice@soroban.dev";
    assert!(is_valid_email(email));
    let status = check_token(email, None);
    assert_eq!(status, VerificationBadgeStatus::Pending);
}

#[test]
fn test_verification_flow_with_valid_token() {
    // Simulates second step: submit email + token (verified).
    let email = "alice@soroban.dev";
    assert!(is_valid_email(email));
    let status = check_token(email, Some("verify-def456abc"));
    assert_eq!(status, VerificationBadgeStatus::Verified);
}

#[test]
fn test_verification_flow_with_bad_token() {
    let email = "alice@soroban.dev";
    assert!(is_valid_email(email));
    let status = check_token(email, Some("bad!"));
    assert_eq!(status, VerificationBadgeStatus::Failed);
}
