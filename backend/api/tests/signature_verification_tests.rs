// Issue #888: runtime verification of the signature crypto core.
//
// Lives in `tests/` so it links the compiled `api` lib and does NOT pull in the
// crate's (pre-existing, unrelated-broken) `#[cfg(test)]` modules. Exercises the
// public `verify_signature` with real Ed25519 and secp256k1 keypairs.

use api::signature_verification::{fingerprint, verify_signature, SigError, SignatureAlgorithm};

#[test]
fn ed25519_real_signature_roundtrip() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let msg = b"deploy:contract-abc:0xdeadbeef";
    let sig = sk.sign(msg);

    assert!(
        verify_signature(
            SignatureAlgorithm::Ed25519,
            vk.as_bytes(),
            msg,
            &sig.to_bytes()
        )
        .is_ok(),
        "valid ed25519 signature must verify"
    );
    assert_eq!(
        verify_signature(
            SignatureAlgorithm::Ed25519,
            vk.as_bytes(),
            b"tampered",
            &sig.to_bytes()
        ),
        Err(SigError::VerificationFailed),
        "tampered message must fail"
    );
}

#[test]
fn ed25519_wrong_key_fails() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let sk = SigningKey::generate(&mut OsRng);
    let other = SigningKey::generate(&mut OsRng);
    let msg = b"message";
    let sig = sk.sign(msg);

    assert!(
        verify_signature(
            SignatureAlgorithm::Ed25519,
            other.verifying_key().as_bytes(),
            msg,
            &sig.to_bytes()
        )
        .is_err(),
        "signature must not verify under a different key"
    );
}

#[test]
fn secp256k1_real_signature_roundtrip_compressed_and_uncompressed() {
    use k256::ecdsa::{signature::Signer, Signature, SigningKey};
    use rand::rngs::OsRng;

    let sk = SigningKey::random(&mut OsRng);
    let vk = sk.verifying_key();
    let msg = b"deploy:contract-xyz:0xc0ffee";
    let sig: Signature = sk.sign(msg);

    // Compressed SEC1 public key (33 bytes).
    let pk_compressed = vk.to_sec1_bytes();
    assert!(
        verify_signature(
            SignatureAlgorithm::Secp256k1,
            &pk_compressed,
            msg,
            &sig.to_bytes()
        )
        .is_ok(),
        "valid secp256k1 signature must verify with a compressed key"
    );

    // Uncompressed SEC1 public key (65 bytes).
    let pk_uncompressed = vk.to_encoded_point(false);
    assert!(
        verify_signature(
            SignatureAlgorithm::Secp256k1,
            pk_uncompressed.as_bytes(),
            msg,
            &sig.to_bytes()
        )
        .is_ok(),
        "valid secp256k1 signature must verify with an uncompressed key"
    );

    assert!(
        verify_signature(
            SignatureAlgorithm::Secp256k1,
            &pk_compressed,
            b"tampered",
            &sig.to_bytes()
        )
        .is_err(),
        "tampered message must fail under secp256k1"
    );
}

#[test]
fn secp256k1_der_signature_is_accepted() {
    use k256::ecdsa::{signature::Signer, Signature, SigningKey};
    use rand::rngs::OsRng;

    let sk = SigningKey::random(&mut OsRng);
    let vk = sk.verifying_key();
    let msg = b"der-encoded-signature";
    let sig: Signature = sk.sign(msg);
    let der = sig.to_der();

    assert!(
        verify_signature(
            SignatureAlgorithm::Secp256k1,
            &vk.to_sec1_bytes(),
            msg,
            der.as_bytes()
        )
        .is_ok(),
        "DER-encoded secp256k1 signature must verify"
    );
}

#[test]
fn cross_algorithm_inputs_are_rejected() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    // An Ed25519 key/signature must not verify as secp256k1.
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let msg = b"m";
    let sig = sk.sign(msg);
    assert!(verify_signature(
        SignatureAlgorithm::Secp256k1,
        vk.as_bytes(),
        msg,
        &sig.to_bytes()
    )
    .is_err());
}

#[test]
fn fingerprint_is_deterministic_and_algorithm_scoped() {
    let pk = [9u8; 33];
    let f1 = fingerprint(SignatureAlgorithm::Secp256k1, &pk);
    let f2 = fingerprint(SignatureAlgorithm::Secp256k1, &pk);
    assert_eq!(f1, f2);
    assert_ne!(f1, fingerprint(SignatureAlgorithm::Ed25519, &pk));
    assert_eq!(f1.len(), 64);
}
