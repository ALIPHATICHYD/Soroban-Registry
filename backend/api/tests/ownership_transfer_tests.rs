// ═══════════════════════════════════════════════════════════════════════════
// SIGNATURE-ANCHORED OWNERSHIP TRANSFER TESTS (Issues #1058, #1094)
// ═══════════════════════════════════════════════════════════════════════════
//
// Two groups of tests live here.
//
// Pure tests exercise the payload builders, the freshness window, and signature
// verification. They need neither a database nor a running API:
//
//   To run: cargo test --test ownership_transfer_tests
//
// Live tests drive the real HTTP endpoints and assert the properties the issue asks for:
// ownership moves only after two verified signatures, an expired request can never be
// accepted (including under a 10-way concurrent race), history is queryable per contract,
// and expired / rejected / double-accept / forged-signature attempts all fail closed.
//
//   To run: cargo test --test ownership_transfer_tests -- --ignored --nocapture
//
// Live tests require:
//   * a running API at TEST_API_BASE_URL (default http://localhost:3001) with a database,
//   * JWT_SECRET set on the API process; without it the AuthClaims extractor returns 500
//     rather than 401 and the authorization assertions will not mean what they say,
//   * ideally OWNERSHIP_TRANSFER_MIN_EXPIRY_SECS=1 on the API process. The expiry tests
//     read min_expiry_window_secs() and wait for however long the server demands, so they
//     are correct either way; at the default they simply take a minute each.
//
// Why everything is an integration test rather than a `#[cfg(test)] mod tests` inside
// src/ownership_transfer.rs: `cargo test -p api --lib` does not compile on this branch for
// reasons unrelated to #1094 (pre-existing errors in other modules' test blocks).
// Integration test targets link the library compiled *without* cfg(test), so this file
// builds and runs regardless. That is also why api::ownership_transfer is `pub`.
//
// Fixture note: the random_strkey helper used elsewhere in this suite produces
// format-valid addresses with no private key behind them, so it cannot sign anything. Here
// each actor is a real ed25519 keypair whose Stellar address is derived from its public
// key, registered through the publisher endpoint, and issued a real JWT
// through the challenge/verify flow. Payloads are built with the same functions the server
// uses, so test and server agree byte for byte by construction rather than by copy.
// ═══════════════════════════════════════════════════════════════════════════

use api::ownership_transfer::{
    check_signature_freshness, decision_payload, initiate_payload, min_expiry_window_secs,
    resolve_algorithm, verify_transfer_signature, TransferDecision, MAX_EXPIRY_WINDOW_SECS,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use reqwest::StatusCode;
use serde_json::{json, Value};
use shared::OwnershipTransferStatus;
use uuid::Uuid;

// ── Keypair and signing helpers ───────────────────────────────────────────────

fn new_keypair() -> (SigningKey, String) {
    let signing = SigningKey::generate(&mut rand::rngs::OsRng);
    // `strkey`'s `to_string` returns a `heapless::String<56>`, not a `std::string::String`.
    let address = stellar_strkey::ed25519::PublicKey(signing.verifying_key().to_bytes())
        .to_string()
        .as_str()
        .to_owned();
    (signing, address)
}

fn sign_b64(signing: &SigningKey, payload: &str) -> String {
    BASE64.encode(signing.sign(payload.as_bytes()).to_bytes())
}

fn random_nonce() -> String {
    Uuid::new_v4().simple().to_string()
}

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// `ApiError`'s fields are private, so read the status the way a client would.
fn status_code_of(err: api::error::ApiError) -> u16 {
    use axum::response::IntoResponse;
    err.into_response().status().as_u16()
}

// ═══════════════════════════════════════════════════════════════════════════
// PURE TESTS: payloads, freshness, verification
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_initiate_payload_is_domain_separated_and_stable() {
    let payload = initiate_payload(
        Uuid::nil(),
        "GFROM",
        "GTO",
        1_800_000_000,
        "nonce1",
        1_700_000_000,
    );

    // Pinned literal, not a reconstruction: this format is a wire contract, and changing it
    // silently invalidates every signature clients have already produced.
    assert_eq!(
        payload,
        "soroban-registry:ownership-transfer:v1:initiate:\
         00000000-0000-0000-0000-000000000000:GFROM:GTO:1800000000:nonce1:1700000000"
    );
}

#[test]
fn test_decision_payload_distinguishes_accept_from_reject() {
    let transfer = Uuid::new_v4();
    let contract = Uuid::new_v4();

    let accept = decision_payload(
        TransferDecision::Accept,
        transfer,
        contract,
        "GFROM",
        "GTO",
        "rnonce",
        "dnonce",
        1_700_000_000,
    );
    let reject = decision_payload(
        TransferDecision::Reject,
        transfer,
        contract,
        "GFROM",
        "GTO",
        "rnonce",
        "dnonce",
        1_700_000_000,
    );

    assert_ne!(
        accept, reject,
        "an acceptance signature must not also be a valid rejection signature"
    );
    assert!(accept.contains(":accept:"));
    assert!(reject.contains(":reject:"));
}

#[test]
fn test_initiate_signature_cannot_be_replayed_as_a_decision() {
    let contract = Uuid::new_v4();
    let (signing, address) = new_keypair();

    let initiate = initiate_payload(contract, &address, "GTO", 1_800_000_000, "n1", now_unix());
    let initiate_sig = sign_b64(&signing, &initiate);

    let decision = decision_payload(
        TransferDecision::Accept,
        Uuid::new_v4(),
        contract,
        &address,
        "GTO",
        "n1",
        "n2",
        now_unix(),
    );

    assert!(
        verify_transfer_signature(&address, &decision, &initiate_sig).is_err(),
        "a phase-1 signature must not verify against a phase-2 payload"
    );
}

#[test]
fn test_valid_signature_verifies() {
    let (signing, address) = new_keypair();
    let payload = initiate_payload(
        Uuid::new_v4(),
        &address,
        "GTO",
        1_800_000_000,
        "n",
        now_unix(),
    );

    verify_transfer_signature(&address, &payload, &sign_b64(&signing, &payload))
        .expect("a signature by the account's own key must verify");
}

#[test]
fn test_forged_signature_fails_verification() {
    let (_owner_key, owner_address) = new_keypair();
    let (attacker_key, _attacker_address) = new_keypair();

    let payload = initiate_payload(
        Uuid::new_v4(),
        &owner_address,
        "GTO",
        1_800_000_000,
        "n",
        now_unix(),
    );
    // A perfectly well-formed signature, just by the wrong key.
    let forged = sign_b64(&attacker_key, &payload);

    let err = verify_transfer_signature(&owner_address, &payload, &forged)
        .expect_err("a signature by a different key must not verify");
    assert_eq!(
        status_code_of(err),
        422,
        "a well-formed but non-verifying signature is a 422, not a 400"
    );
}

#[test]
fn test_tampered_payload_fails_verification() {
    let (signing, address) = new_keypair();
    let contract = Uuid::new_v4();
    let (_, recipient) = new_keypair();
    let (_, attacker) = new_keypair();
    let signed_at = now_unix();

    let signed = initiate_payload(
        contract,
        &address,
        &recipient,
        1_800_000_000,
        "n",
        signed_at,
    );
    let signature = sign_b64(&signing, &signed);

    // The same signature, but the recipient has been swapped for the attacker.
    let tampered = initiate_payload(contract, &address, &attacker, 1_800_000_000, "n", signed_at);

    assert!(
        verify_transfer_signature(&address, &tampered, &signature).is_err(),
        "redirecting the transfer to a different recipient must invalidate the signature"
    );
}

#[test]
fn test_malformed_signature_encoding_is_rejected() {
    let (_signing, address) = new_keypair();
    let payload = initiate_payload(
        Uuid::new_v4(),
        &address,
        "GTO",
        1_800_000_000,
        "n",
        now_unix(),
    );

    let err = verify_transfer_signature(&address, &payload, "not base64 at all !!")
        .expect_err("an undecodable signature must be rejected");
    assert_eq!(
        status_code_of(err),
        400,
        "an undecodable signature is a client encoding error"
    );
}

#[test]
fn test_wrong_length_signature_is_rejected() {
    let (_signing, address) = new_keypair();
    let payload = initiate_payload(
        Uuid::new_v4(),
        &address,
        "GTO",
        1_800_000_000,
        "n",
        now_unix(),
    );

    let err = verify_transfer_signature(&address, &payload, &BASE64.encode([7u8; 32]))
        .expect_err("a 32-byte signature is not an ed25519 signature");
    assert_eq!(status_code_of(err), 400);
}

#[test]
fn test_stale_and_future_signed_at_are_rejected() {
    let now = 1_700_000_000i64;

    check_signature_freshness(now, now).expect("a signature made now must be accepted");
    check_signature_freshness(now - 60, now).expect("a one-minute-old signature is fine");

    let stale = check_signature_freshness(now - 86_400, now)
        .expect_err("a day-old signature must be rejected");
    assert_eq!(status_code_of(stale), 422);

    let future = check_signature_freshness(now + 86_400, now)
        .expect_err("a signature dated far in the future must be rejected");
    assert_eq!(status_code_of(future), 422);
}

#[test]
fn test_resolve_algorithm_accepts_only_ed25519() {
    assert_eq!(resolve_algorithm(None).unwrap(), "ed25519");
    assert_eq!(resolve_algorithm(Some("")).unwrap(), "ed25519");
    assert_eq!(resolve_algorithm(Some("ed25519")).unwrap(), "ed25519");
    assert_eq!(resolve_algorithm(Some("ED25519")).unwrap(), "ed25519");

    // Stellar accounts are ed25519, so a secp256k1 signature could never be checked against
    // publishers.stellar_address.
    let err = resolve_algorithm(Some("secp256k1"))
        .expect_err("secp256k1 must not be accepted for ownership transfers");
    assert_eq!(status_code_of(err), 400);
}

#[test]
fn test_status_serialises_as_snake_case() {
    // Regression for #1058: Serialize was derived with no rename, so the wire value was
    // "Pending" while the database column held 'pending'.
    assert_eq!(
        serde_json::to_string(&OwnershipTransferStatus::Pending).unwrap(),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&OwnershipTransferStatus::Completed).unwrap(),
        "\"completed\""
    );
    assert_eq!(
        serde_json::from_str::<OwnershipTransferStatus>("\"expired\"").unwrap(),
        OwnershipTransferStatus::Expired
    );

    assert_eq!(OwnershipTransferStatus::Pending.as_str(), "pending");
    assert_eq!(
        OwnershipTransferStatus::parse("completed"),
        Some(OwnershipTransferStatus::Completed)
    );
    assert_eq!(OwnershipTransferStatus::parse("nonsense"), None);
    assert!(OwnershipTransferStatus::Pending.is_live());
    assert!(!OwnershipTransferStatus::Expired.is_live());
    assert!(!OwnershipTransferStatus::Completed.is_live());
}

#[test]
fn test_expiry_window_bounds_are_sane() {
    assert!(min_expiry_window_secs() > 0);
    assert!(
        MAX_EXPIRY_WINDOW_SECS > min_expiry_window_secs(),
        "the permitted expiry range must be non-empty"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// LIVE HTTP FIXTURE
// ═══════════════════════════════════════════════════════════════════════════

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

/// A publisher backed by a real keypair, registered in the database, holding a real JWT.
struct Actor {
    signing: SigningKey,
    address: String,
    token: String,
}

fn random_contract_strkey() -> String {
    let raw = format!(
        "{:032X}{:032X}",
        Uuid::new_v4().as_u128(),
        Uuid::new_v4().as_u128()
    );
    format!("C{}", &raw[..55])
}

/// A minimal but genuinely valid wasm module, unique on every call.
///
/// This matters more than it looks. Publishing with no artifact leaves
/// `artifact_scan_status = 'pending'`, and `publish_contract` then stores the contract as
/// `private`, which makes `GET /api/contracts/:id` return 403 to everyone including the
/// publisher itself. Supplying an artifact the scanner passes is what makes the contract
/// public, which is what lets these tests read the owner back over HTTP and prove ownership
/// moved. The single custom section exists only to give each artifact a distinct hash.
fn unique_wasm_module() -> Vec<u8> {
    let name = Uuid::new_v4().simple().to_string().into_bytes();
    let mut wasm = Vec::with_capacity(11 + name.len());
    wasm.extend_from_slice(b"\0asm");
    wasm.extend_from_slice(&[1, 0, 0, 0]);
    wasm.push(0x00); // custom section id
    wasm.push((name.len() + 1) as u8); // section size
    wasm.push(name.len() as u8); // name length
    wasm.extend_from_slice(&name);
    wasm
}

/// Register the publisher identity used by the auth challenge.
async fn register_publisher(client: &reqwest::Client, base: &str, address: &str) {
    let res = client
        .post(format!("{}/api/publishers", base))
        .json(&json!({
            "id": Uuid::nil(),
            "stellar_address": address,
            "username": null,
            "email": null,
            "github_url": null,
            "website": null,
            "created_at": chrono::Utc::now(),
        }))
        .send()
        .await
        .expect("publisher registration failed");
    assert!(
        res.status().is_success(),
        "publisher registration failed: {}",
        res.text().await.unwrap_or_default()
    );
}

/// Publish a contract as an authenticated, registered publisher.
async fn publish_contract(
    client: &reqwest::Client,
    base: &str,
    address: &str,
    token: &str,
) -> Value {
    use sha2::{Digest, Sha256};

    let wasm = unique_wasm_module();
    let wasm_hash = hex::encode(Sha256::digest(&wasm));

    let res = client
        .post(format!("{}/api/contracts", base))
        .bearer_auth(token)
        .json(&json!({
            "contract_id": random_contract_strkey(),
            "wasm_hash": wasm_hash,
            "wasm_artifact_base64": BASE64.encode(&wasm),
            "name": format!("Transfer Test {}", Uuid::new_v4()),
            "network": "testnet",
            "publisher_address": address,
            "tags": []
        }))
        .send()
        .await
        .expect("publish request failed");

    let status = res.status();
    let body: Value = res.json().await.expect("publish response was not JSON");
    assert_eq!(status, StatusCode::OK, "publish failed: {}", body);
    assert_eq!(
        body["visibility"], "public",
        "the fixture depends on the published contract being publicly readable: {}",
        body
    );
    body
}

/// Complete the challenge/verify handshake and return a bearer token.
async fn mint_token(
    client: &reqwest::Client,
    base: &str,
    signing: &SigningKey,
    address: &str,
) -> String {
    let challenge: Value = client
        .get(format!("{}/api/auth/challenge", base))
        .query(&[("address", address)])
        .send()
        .await
        .expect("challenge request failed")
        .json()
        .await
        .expect("challenge response was not JSON");

    let nonce = challenge["nonce"]
        .as_str()
        .unwrap_or_else(|| panic!("challenge response had no nonce: {}", challenge));

    // The challenge handshake signs the raw nonce and hex-encodes both key and signature,
    // which is a different encoding from the transfer payloads (base64). That is the
    // existing /api/auth/verify contract, not an inconsistency introduced here.
    let res = client
        .post(format!("{}/api/auth/verify", base))
        .json(&json!({
            "address": address,
            "public_key": hex::encode(signing.verifying_key().to_bytes()),
            "signature": hex::encode(signing.sign(nonce.as_bytes()).to_bytes()),
        }))
        .send()
        .await
        .expect("verify request failed");

    let status = res.status();
    let body: Value = res.json().await.expect("verify response was not JSON");
    assert_eq!(
        status,
        StatusCode::OK,
        "auth verify failed (is JWT_SECRET set on the API?): {}",
        body
    );

    body["token"]
        .as_str()
        .unwrap_or_else(|| panic!("verify response had no token: {}", body))
        .to_string()
}

/// A registered, authenticated actor plus the uuid of a contract it owns.
async fn new_actor_with_contract(client: &reqwest::Client, base: &str) -> (Actor, Uuid) {
    let (signing, address) = new_keypair();
    register_publisher(client, base, &address).await;
    let token = mint_token(client, base, &signing, &address).await;
    let contract = publish_contract(client, base, &address, &token).await;
    let contract_id = Uuid::parse_str(contract["id"].as_str().expect("contract id"))
        .expect("contract id was not a uuid");
    (
        Actor {
            signing,
            address,
            token,
        },
        contract_id,
    )
}

/// A registered, authenticated actor that owns some unrelated contract.
async fn new_actor(client: &reqwest::Client, base: &str) -> Actor {
    new_actor_with_contract(client, base).await.0
}

struct Scenario {
    client: reqwest::Client,
    base: String,
    owner: Actor,
    recipient: Actor,
    contract_id: Uuid,
}

async fn scenario() -> Scenario {
    let client = reqwest::Client::new();
    let base = api_base_url();
    let (owner, contract_id) = new_actor_with_contract(&client, &base).await;
    let recipient = new_actor(&client, &base).await;
    Scenario {
        client,
        base,
        owner,
        recipient,
        contract_id,
    }
}

/// A signed create body, plus the nonce it used (phase 2 has to reference it).
fn create_body(
    owner: &Actor,
    recipient_address: &str,
    contract_id: Uuid,
    ttl_secs: i64,
) -> (Value, String) {
    let nonce = random_nonce();
    let signed_at = now_unix();
    let expires_at = signed_at + ttl_secs;
    let payload = initiate_payload(
        contract_id,
        &owner.address,
        recipient_address,
        expires_at,
        &nonce,
        signed_at,
    );

    (
        json!({
            "to_publisher_address": recipient_address,
            "expires_at_unix": expires_at,
            "nonce": nonce,
            "signed_at_unix": signed_at,
            "signature": sign_b64(&owner.signing, &payload),
        }),
        nonce,
    )
}

fn decision_body(
    signer: &SigningKey,
    accept: bool,
    transfer_id: Uuid,
    contract_id: Uuid,
    from_address: &str,
    to_address: &str,
    request_nonce: &str,
) -> Value {
    let nonce = random_nonce();
    let signed_at = now_unix();
    let payload = decision_payload(
        TransferDecision::from_accept_flag(accept),
        transfer_id,
        contract_id,
        from_address,
        to_address,
        request_nonce,
        &nonce,
        signed_at,
    );

    json!({
        "accept": accept,
        "nonce": nonce,
        "signed_at_unix": signed_at,
        "signature": sign_b64(signer, &payload),
    })
}

/// The recipient's acceptance of `transfer_id`, freshly signed.
fn accept_body(sc: &Scenario, transfer_id: Uuid, request_nonce: &str) -> Value {
    decision_body(
        &sc.recipient.signing,
        true,
        transfer_id,
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        request_nonce,
    )
}

/// The recipient's rejection of `transfer_id`, freshly signed.
fn reject_body(sc: &Scenario, transfer_id: Uuid, request_nonce: &str) -> Value {
    decision_body(
        &sc.recipient.signing,
        false,
        transfer_id,
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        request_nonce,
    )
}

async fn post_create(sc: &Scenario, token: Option<&str>, body: &Value) -> (StatusCode, Value) {
    let mut req = sc.client.post(format!(
        "{}/api/contracts/{}/ownership-transfer",
        sc.base, sc.contract_id
    ));
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }
    let res = req.json(body).send().await.expect("create request failed");
    let status = res.status();
    let body = res.json().await.unwrap_or(Value::Null);
    (status, body)
}

async fn post_confirm(
    sc: &Scenario,
    transfer_id: Uuid,
    token: Option<&str>,
    body: &Value,
) -> (StatusCode, Value) {
    let mut req = sc.client.post(format!(
        "{}/api/ownership-transfers/{}/confirm",
        sc.base, transfer_id
    ));
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }
    let res = req.json(body).send().await.expect("confirm request failed");
    let status = res.status();
    let body = res.json().await.unwrap_or(Value::Null);
    (status, body)
}

async fn get_json(client: &reqwest::Client, url: String) -> Value {
    client
        .get(&url)
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET {} failed: {}", url, e))
        .json()
        .await
        .unwrap_or_else(|e| panic!("GET {} returned non-JSON: {}", url, e))
}

async fn current_owner(sc: &Scenario) -> String {
    let contract = get_json(
        &sc.client,
        format!("{}/api/contracts/{}", sc.base, sc.contract_id),
    )
    .await;
    contract["publisher_id"]
        .as_str()
        .unwrap_or_else(|| panic!("contract response had no publisher_id: {}", contract))
        .to_string()
}

async fn get_transfer(sc: &Scenario, transfer_id: Uuid) -> Value {
    get_json(
        &sc.client,
        format!("{}/api/ownership-transfers/{}", sc.base, transfer_id),
    )
    .await
}

async fn get_logs(sc: &Scenario, transfer_id: Uuid) -> Vec<Value> {
    get_json(
        &sc.client,
        format!("{}/api/ownership-transfers/{}/logs", sc.base, transfer_id),
    )
    .await
    .as_array()
    .cloned()
    .unwrap_or_default()
}

// A note on the error codes asserted below. The specific name a handler passes to
// `ApiError::conflict("TransferExpired", ...)` lands in the envelope's `code` field, and
// `error::normalize_error_code` is supposed to convert it to SCREAMING_SNAKE_CASE. It does
// not: the branch that would insert an underscore tests whether the previous character of
// the *already uppercased* output is lowercase, which it never is. So PascalCase names come
// out squashed, e.g. TRANSFEREXPIRED. These tests assert what the API actually emits. The
// handlers keep passing PascalCase to stay consistent with the ~100 other handler modules;
// fixing the normaliser would change every error code the API emits and belongs in its own
// change.
fn log_actions(logs: &[Value]) -> Vec<String> {
    logs.iter()
        .map(|l| l["action"].as_str().unwrap_or("").to_string())
        .collect()
}

/// Open a transfer and return `(transfer_id, request_nonce)`.
async fn open_transfer(sc: &Scenario, ttl_secs: i64) -> (Uuid, String) {
    let (body, nonce) = create_body(&sc.owner, &sc.recipient.address, sc.contract_id, ttl_secs);
    let (status, created) = post_create(sc, Some(&sc.owner.token), &body).await;
    assert_eq!(status, StatusCode::OK, "create failed: {}", created);
    let id = Uuid::parse_str(created["id"].as_str().expect("transfer id")).unwrap();
    (id, nonce)
}

/// The shortest transfer the server will accept, and how long to wait for it to lapse.
fn short_ttl() -> (i64, std::time::Duration) {
    let ttl = min_expiry_window_secs();
    (ttl, std::time::Duration::from_secs(ttl as u64 + 2))
}

// ═══════════════════════════════════════════════════════════════════════════
// AC1: ownership changes only after both signatures are verified
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_two_phase_transfer_moves_ownership_only_after_both_signatures() {
    let sc = scenario().await;
    let owner_before = current_owner(&sc).await;

    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    // One signature: the request exists, ownership has not moved.
    let pending = get_transfer(&sc, transfer_id).await;
    assert_eq!(pending["status"], "pending");
    assert_eq!(pending["from_confirmation"], true);
    assert_eq!(pending["to_confirmation"], false);
    assert!(
        pending["from_signature"].is_string(),
        "the sender's signature must be recorded on the row"
    );
    assert!(pending["decision_signature"].is_null());
    assert_eq!(
        current_owner(&sc).await,
        owner_before,
        "ownership must not move on a one-sided request"
    );

    // Second signature: ownership moves.
    let body = accept_body(&sc, transfer_id, &request_nonce);
    let (status, completed) =
        post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &body).await;
    assert_eq!(status, StatusCode::OK, "accept failed: {}", completed);
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["to_confirmation"], true);

    let owner_after = current_owner(&sc).await;
    assert_ne!(owner_after, owner_before, "ownership should have moved");
    assert_eq!(
        owner_after,
        completed["to_publisher_id"].as_str().unwrap(),
        "the contract must now be owned by the transfer recipient"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2: expired requests cannot be accepted, including under a race
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_expired_transfer_cannot_be_accepted() {
    let sc = scenario().await;
    let (ttl, wait) = short_ttl();
    let (transfer_id, request_nonce) = open_transfer(&sc, ttl).await;

    tokio::time::sleep(wait).await;

    let body = accept_body(&sc, transfer_id, &request_nonce);
    let (status, err) = post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &body).await;

    assert_eq!(status, StatusCode::CONFLICT, "expected a 409, got {}", err);
    assert_eq!(err["code"], "TRANSFEREXPIRED");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_expired_accept_leaves_no_partial_transfer() {
    let sc = scenario().await;
    let owner_before = current_owner(&sc).await;
    let (ttl, wait) = short_ttl();
    let (transfer_id, request_nonce) = open_transfer(&sc, ttl).await;

    tokio::time::sleep(wait).await;

    let body = accept_body(&sc, transfer_id, &request_nonce);
    post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &body).await;

    // The transfer must be terminally expired, the contract untouched, and the history must
    // not claim a completion. A partial write shows up as exactly one of these three
    // disagreeing with the other two.
    let row = get_transfer(&sc, transfer_id).await;
    assert_eq!(row["status"], "expired");
    assert_eq!(row["to_confirmation"], false);
    assert!(row["decision_signature"].is_null());
    assert_eq!(current_owner(&sc).await, owner_before);

    let actions = log_actions(&get_logs(&sc, transfer_id).await);
    assert!(actions.contains(&"transfer_expired".to_string()));
    assert!(
        !actions.contains(&"transfer_completed".to_string()),
        "history must not record a completion for an expired transfer: {:?}",
        actions
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_concurrent_accepts_complete_exactly_once() {
    let sc = scenario().await;
    let owner_before = current_owner(&sc).await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    // Ten independently signed acceptances, each with its own nonce, fired at once. Exactly
    // one must win, and ownership must move exactly once.
    let mut handles = Vec::new();
    for _ in 0..10 {
        let client = sc.client.clone();
        let base = sc.base.clone();
        let token = sc.recipient.token.clone();
        let body = accept_body(&sc, transfer_id, &request_nonce);
        handles.push(tokio::spawn(async move {
            let res = client
                .post(format!(
                    "{}/api/ownership-transfers/{}/confirm",
                    base, transfer_id
                ))
                .bearer_auth(token)
                .json(&body)
                .send()
                .await
                .expect("concurrent confirm request failed");
            let status = res.status();
            let body: Value = res.json().await.unwrap_or(Value::Null);
            (status, body)
        }));
    }

    let mut accepted = 0;
    let mut conflicted = 0;
    for handle in handles {
        let (status, body) = handle.await.expect("confirm task panicked");
        match status {
            StatusCode::OK => accepted += 1,
            StatusCode::CONFLICT => conflicted += 1,
            other => panic!(
                "unexpected status {} from a concurrent accept: {}",
                other, body
            ),
        }
    }

    assert_eq!(accepted, 1, "exactly one acceptance may succeed");
    assert_eq!(
        conflicted, 9,
        "the other nine must be rejected as conflicts"
    );

    let row = get_transfer(&sc, transfer_id).await;
    assert_eq!(row["status"], "completed");

    let actions = log_actions(&get_logs(&sc, transfer_id).await);
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "transfer_completed")
            .count(),
        1,
        "history must record exactly one completion: {:?}",
        actions
    );

    let owner_after = current_owner(&sc).await;
    assert_ne!(owner_after, owner_before);
    assert_eq!(owner_after, row["to_publisher_id"].as_str().unwrap());
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_double_accept_returns_conflict() {
    let sc = scenario().await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    let first = accept_body(&sc, transfer_id, &request_nonce);
    let (status, body) = post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &first).await;
    assert_eq!(status, StatusCode::OK, "first accept failed: {}", body);

    // A freshly signed, entirely valid second acceptance still must not apply.
    let second = accept_body(&sc, transfer_id, &request_nonce);
    let (status, err) = post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &second).await;
    assert_eq!(status, StatusCode::CONFLICT, "expected a 409, got {}", err);
}

// ═══════════════════════════════════════════════════════════════════════════
// AC3: history and provenance
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_transfer_history_is_queryable_per_contract() {
    let sc = scenario().await;

    // A rejected attempt, then a completed one. Both must remain visible: the history is
    // append-only, so a rejection is not erased by a later success.
    let (rejected_id, rejected_nonce) = open_transfer(&sc, 3_600).await;
    let reject = reject_body(&sc, rejected_id, &rejected_nonce);
    let (status, body) = post_confirm(&sc, rejected_id, Some(&sc.recipient.token), &reject).await;
    assert_eq!(status, StatusCode::OK, "reject failed: {}", body);

    let (completed_id, completed_nonce) = open_transfer(&sc, 3_600).await;
    let accept = accept_body(&sc, completed_id, &completed_nonce);
    let (status, body) = post_confirm(&sc, completed_id, Some(&sc.recipient.token), &accept).await;
    assert_eq!(status, StatusCode::OK, "accept failed: {}", body);

    let history = get_json(
        &sc.client,
        format!(
            "{}/api/contracts/{}/ownership-transfer",
            sc.base, sc.contract_id
        ),
    )
    .await;
    let rows = history.as_array().expect("history should be an array");
    assert_eq!(rows.len(), 2, "both attempts must be retained: {}", history);

    let pairs: Vec<(String, String)> = rows
        .iter()
        .map(|r| {
            (
                r["id"].as_str().unwrap_or_default().to_string(),
                r["status"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    assert!(
        pairs
            .iter()
            .any(|(id, st)| id == &rejected_id.to_string() && st == "rejected"),
        "the rejected attempt must still be in the history: {:?}",
        pairs
    );
    assert!(
        pairs
            .iter()
            .any(|(id, st)| id == &completed_id.to_string() && st == "completed"),
        "the completed transfer must be in the history: {:?}",
        pairs
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_stored_signatures_verify_offline() {
    let sc = scenario().await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;
    let body = accept_body(&sc, transfer_id, &request_nonce);
    let (status, completed) =
        post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &body).await;
    assert_eq!(status, StatusCode::OK, "accept failed: {}", completed);

    // This is the point of storing the payloads: a third party can take the row and check
    // both signatures with nothing but ed25519 and the Stellar addresses. Deliberately no
    // call into the crate's verification code, so the check is genuinely independent.
    for (payload_field, signature_field, address_field) in [
        (
            "from_signed_payload",
            "from_signature",
            "from_signer_address",
        ),
        (
            "decision_signed_payload",
            "decision_signature",
            "decision_signer_address",
        ),
    ] {
        let payload = completed[payload_field]
            .as_str()
            .unwrap_or_else(|| panic!("{} missing: {}", payload_field, completed));
        let signature_b64 = completed[signature_field]
            .as_str()
            .unwrap_or_else(|| panic!("{} missing", signature_field));
        let address = completed[address_field]
            .as_str()
            .unwrap_or_else(|| panic!("{} missing", address_field));

        let key_bytes = stellar_strkey::ed25519::PublicKey::from_string(address)
            .expect("stored signer address must be a valid strkey")
            .0;
        let verifying = VerifyingKey::from_bytes(&key_bytes).expect("valid ed25519 key");
        let signature_bytes: [u8; 64] = BASE64
            .decode(signature_b64)
            .expect("stored signature must be base64")
            .try_into()
            .expect("stored signature must be 64 bytes");

        verifying
            .verify_strict(
                payload.as_bytes(),
                &ed25519_dalek::Signature::from_bytes(&signature_bytes),
            )
            .unwrap_or_else(|e| panic!("stored {} did not verify offline: {}", signature_field, e));
    }

    // And the two signatures come from genuinely different accounts (invariant I1).
    assert_ne!(
        completed["from_signer_address"], completed["decision_signer_address"],
        "a completed transfer must carry signatures from two distinct accounts"
    );
    assert_eq!(completed["from_signer_address"], json!(sc.owner.address));
    assert_eq!(
        completed["decision_signer_address"],
        json!(sc.recipient.address)
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_transfer_logs_contain_full_provenance_chain() {
    let sc = scenario().await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;
    let body = accept_body(&sc, transfer_id, &request_nonce);
    post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &body).await;

    let logs = get_logs(&sc, transfer_id).await;
    assert_eq!(
        log_actions(&logs),
        vec!["transfer_request_created", "transfer_completed"],
        "the chain must record both phases, in order"
    );

    for entry in &logs {
        assert_eq!(entry["actor_type"], "publisher");
        assert!(
            entry["actor_id"].is_string(),
            "a publisher action must name the acting publisher: {}",
            entry
        );
        assert!(
            entry["details"]["signed_payload"].is_string(),
            "each phase must record what was actually signed: {}",
            entry
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AC4: rejected, forged, and unauthorized attempts
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_rejected_transfer_cannot_be_accepted() {
    let sc = scenario().await;
    let owner_before = current_owner(&sc).await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    let reject = reject_body(&sc, transfer_id, &request_nonce);
    let (status, rejected) =
        post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &reject).await;
    assert_eq!(status, StatusCode::OK, "reject failed: {}", rejected);
    assert_eq!(rejected["status"], "rejected");

    let accept = accept_body(&sc, transfer_id, &request_nonce);
    let (status, err) = post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &accept).await;
    assert_eq!(status, StatusCode::CONFLICT, "expected a 409, got {}", err);
    assert_eq!(
        current_owner(&sc).await,
        owner_before,
        "a rejected transfer must never move ownership"
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_accept_with_wrong_key_signature_is_rejected() {
    let sc = scenario().await;
    let owner_before = current_owner(&sc).await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    // The recipient's own JWT, but the payload is signed by somebody else's key. Proving
    // session control must not be sufficient on its own.
    let (attacker_key, _) = new_keypair();
    let body = decision_body(
        &attacker_key,
        true,
        transfer_id,
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        &request_nonce,
    );
    let (status, err) = post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &body).await;

    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "expected a 422, got {}",
        err
    );
    assert_eq!(err["code"], "SIGNATUREVERIFICATIONFAILED");

    let row = get_transfer(&sc, transfer_id).await;
    assert_eq!(
        row["status"], "pending",
        "a forged signature must leave the transfer untouched"
    );
    assert_eq!(row["to_confirmation"], false);
    assert_eq!(current_owner(&sc).await, owner_before);
    assert_eq!(
        log_actions(&get_logs(&sc, transfer_id).await),
        vec!["transfer_request_created"],
        "a forged attempt must not append to the history"
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_accept_without_jwt_is_unauthorized() {
    let sc = scenario().await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    let body = accept_body(&sc, transfer_id, &request_nonce);
    let (status, _) = post_confirm(&sc, transfer_id, None, &body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_reject_by_third_party_is_forbidden() {
    // Regression for #1058: the is_from/is_to check lived only inside the accept branch, so
    // any caller at all could reject any pending transfer.
    let sc = scenario().await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;
    let stranger = new_actor(&sc.client, &sc.base).await;

    let body = decision_body(
        &stranger.signing,
        false,
        transfer_id,
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        &request_nonce,
    );
    let (status, err) = post_confirm(&sc, transfer_id, Some(&stranger.token), &body).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "expected a 403, got {}", err);
    assert_eq!(get_transfer(&sc, transfer_id).await["status"], "pending");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_accept_by_sender_is_forbidden() {
    // The sender's consent is already the phase-1 signature. Letting the sender also accept
    // would collapse two-party confirmation back into a single-party ownership change.
    let sc = scenario().await;
    let owner_before = current_owner(&sc).await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    let body = decision_body(
        &sc.owner.signing,
        true,
        transfer_id,
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        &request_nonce,
    );
    let (status, err) = post_confirm(&sc, transfer_id, Some(&sc.owner.token), &body).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "expected a 403, got {}", err);
    assert_eq!(current_owner(&sc).await, owner_before);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_sender_can_cancel_pending_transfer() {
    // Documented assumption: the sender may withdraw a mistaken transfer with a signed
    // rejection rather than waiting for it to expire.
    let sc = scenario().await;
    let (transfer_id, request_nonce) = open_transfer(&sc, 3_600).await;

    let body = decision_body(
        &sc.owner.signing,
        false,
        transfer_id,
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        &request_nonce,
    );
    let (status, cancelled) = post_confirm(&sc, transfer_id, Some(&sc.owner.token), &body).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "sender cancel failed: {}",
        cancelled
    );
    assert_eq!(cancelled["status"], "rejected");
    assert_eq!(
        cancelled["decision_signer_address"],
        json!(sc.owner.address)
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_create_by_non_owner_is_forbidden() {
    let sc = scenario().await;
    let stranger = new_actor(&sc.client, &sc.base).await;

    // A stranger signing correctly with their own key, for a contract they do not own.
    let nonce = random_nonce();
    let signed_at = now_unix();
    let expires_at = signed_at + 3_600;
    let payload = initiate_payload(
        sc.contract_id,
        &stranger.address,
        &sc.recipient.address,
        expires_at,
        &nonce,
        signed_at,
    );
    let body = json!({
        "to_publisher_address": sc.recipient.address,
        "expires_at_unix": expires_at,
        "nonce": nonce,
        "signed_at_unix": signed_at,
        "signature": sign_b64(&stranger.signing, &payload),
    });

    let (status, err) = post_create(&sc, Some(&stranger.token), &body).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "expected a 403, got {}", err);
    assert_eq!(err["code"], "NOTCONTRACTOWNER");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_create_without_jwt_is_unauthorized() {
    let sc = scenario().await;
    let (body, _) = create_body(&sc.owner, &sc.recipient.address, sc.contract_id, 3_600);
    let (status, _) = post_create(&sc, None, &body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_duplicate_pending_transfer_returns_409_not_500() {
    // Regression for #1058: the duplicate branch wrote a history row against a random
    // Uuid::new_v4() with no parent transfer, so the foreign key fired and the caller got a
    // 500 instead of the intended conflict.
    let sc = scenario().await;
    let (_first_id, _) = open_transfer(&sc, 3_600).await;

    let (body, _) = create_body(&sc.owner, &sc.recipient.address, sc.contract_id, 3_600);
    let (status, err) = post_create(&sc, Some(&sc.owner.token), &body).await;

    assert_eq!(status, StatusCode::CONFLICT, "expected a 409, got {}", err);
    assert_eq!(err["code"], "DUPLICATETRANSFER");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_replayed_create_body_is_rejected_by_nonce() {
    let sc = scenario().await;
    let (body, request_nonce) =
        create_body(&sc.owner, &sc.recipient.address, sc.contract_id, 3_600);

    let (status, created) = post_create(&sc, Some(&sc.owner.token), &body).await;
    assert_eq!(status, StatusCode::OK, "create failed: {}", created);
    let transfer_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();

    // Clear the live-transfer conflict so the nonce is the only thing left that can stop a
    // replay: reject the first transfer, then resend the identical create body.
    let reject = reject_body(&sc, transfer_id, &request_nonce);
    let (status, rejected) =
        post_confirm(&sc, transfer_id, Some(&sc.recipient.token), &reject).await;
    assert_eq!(status, StatusCode::OK, "reject failed: {}", rejected);

    let (status, err) = post_create(&sc, Some(&sc.owner.token), &body).await;
    assert_eq!(status, StatusCode::CONFLICT, "expected a 409, got {}", err);
    assert_eq!(err["code"], "NONCEALREADYUSED");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_self_transfer_is_rejected() {
    let sc = scenario().await;
    let (body, _) = create_body(&sc.owner, &sc.owner.address, sc.contract_id, 3_600);
    let (status, err) = post_create(&sc, Some(&sc.owner.token), &body).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "expected a 400, got {}",
        err
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_unregistered_recipient_is_not_found() {
    let sc = scenario().await;
    let (_unknown_key, unknown_address) = new_keypair();
    let (body, _) = create_body(&sc.owner, &unknown_address, sc.contract_id, 3_600);
    let (status, err) = post_create(&sc, Some(&sc.owner.token), &body).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "expected a 404, got {}", err);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_expiry_beyond_the_maximum_window_is_rejected() {
    let sc = scenario().await;
    let (body, _) = create_body(
        &sc.owner,
        &sc.recipient.address,
        sc.contract_id,
        MAX_EXPIRY_WINDOW_SECS + 3_600,
    );
    let (status, err) = post_create(&sc, Some(&sc.owner.token), &body).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "expected a 400, got {}",
        err
    );
    assert_eq!(err["code"], "INVALIDEXPIRY");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_stale_signature_is_rejected_at_the_endpoint() {
    let sc = scenario().await;

    // Correctly signed by the right key, but dated a day ago.
    let nonce = random_nonce();
    let signed_at = now_unix() - 86_400;
    let expires_at = now_unix() + 3_600;
    let payload = initiate_payload(
        sc.contract_id,
        &sc.owner.address,
        &sc.recipient.address,
        expires_at,
        &nonce,
        signed_at,
    );
    let body = json!({
        "to_publisher_address": sc.recipient.address,
        "expires_at_unix": expires_at,
        "nonce": nonce,
        "signed_at_unix": signed_at,
        "signature": sign_b64(&sc.owner.signing, &payload),
    });

    let (status, err) = post_create(&sc, Some(&sc.owner.token), &body).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "expected a 422, got {}",
        err
    );
    assert_eq!(err["code"], "SIGNATUREEXPIRED");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_expired_transfer_is_reported_as_expired_on_read() {
    // Lazy expiry: a read must never present a past-due transfer as still actionable, even
    // if the background sweeper has not run yet.
    let sc = scenario().await;
    let (ttl, wait) = short_ttl();
    let (transfer_id, _) = open_transfer(&sc, ttl).await;

    assert_eq!(get_transfer(&sc, transfer_id).await["status"], "pending");

    tokio::time::sleep(wait).await;

    assert_eq!(
        get_transfer(&sc, transfer_id).await["status"],
        "expired",
        "reading a past-due transfer must expire it"
    );
    assert!(
        log_actions(&get_logs(&sc, transfer_id).await).contains(&"transfer_expired".to_string())
    );
}
