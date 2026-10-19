use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::{PeerFingerprint, PeerPublicKey};
use mm_ports::{PairingIdentityPort, PortError};

const RFC8032_PUBLIC_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const RFC8032_EMPTY_MESSAGE_SIGNATURE: &str = concat!(
    "e5564300c360ac729086e2cc806e828a",
    "84877f1eb8e5d974d873e06522490155",
    "5fb8821590a33bacc61e39701cf9b46",
    "bd25bf5f0595bbe24655141438e7a100b",
);

fn decode_hex(encoded: &str) -> Vec<u8> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for index in (0..bytes.len()).step_by(2) {
        let high = char::from(bytes[index]).to_digit(16).unwrap() as u8;
        let low = char::from(bytes[index + 1]).to_digit(16).unwrap() as u8;
        decoded.push((high << 4) | low);
    }
    decoded
}

fn public_key() -> PeerPublicKey {
    PeerPublicKey::new(decode_hex(RFC8032_PUBLIC_KEY)).unwrap()
}

fn provider() -> Ed25519PairingIdentity {
    Ed25519PairingIdentity
}

#[test]
fn accepts_the_rfc_8032_empty_message_vector() {
    provider()
        .verify_signature(
            &public_key(),
            b"",
            &decode_hex(RFC8032_EMPTY_MESSAGE_SIGNATURE),
        )
        .unwrap();
}

#[test]
fn fingerprints_the_exact_raw_public_key_bytes() {
    let fingerprint = provider().fingerprint_public_key(&public_key()).unwrap();

    assert_eq!(
        fingerprint,
        PeerFingerprint::from_hex(
            "21fe31dfa154a261626bf854046fd2271b7bed4b6abe45aa58877ef47f9721b9"
        )
        .unwrap()
    );
}

#[test]
fn rejects_a_modified_message_or_signature() {
    let identity = provider();
    let key = public_key();
    let signature = decode_hex(RFC8032_EMPTY_MESSAGE_SIGNATURE);

    assert_eq!(
        identity.verify_signature(&key, b"changed", &signature),
        Err(PortError::IdentityRejected)
    );

    let mut changed_signature = signature;
    changed_signature[0] ^= 1;
    assert_eq!(
        identity.verify_signature(&key, b"", &changed_signature),
        Err(PortError::IdentityRejected)
    );
}

#[test]
fn rejects_wrong_key_and_signature_lengths() {
    let identity = provider();
    let short_key = PeerPublicKey::new(vec![0x42; 31]).unwrap();

    assert_eq!(
        identity.fingerprint_public_key(&short_key),
        Err(PortError::IdentityRejected)
    );
    assert_eq!(
        identity.verify_signature(&public_key(), b"", &[0; 63]),
        Err(PortError::IdentityRejected)
    );
}

#[test]
fn rejects_noncanonical_public_key_encodings() {
    let identity = provider();
    let mut noncanonical_y_equals_two = [0xff; 32];
    noncanonical_y_equals_two[0] = 0xef;
    noncanonical_y_equals_two[31] = 0x7f;
    let key = PeerPublicKey::new(noncanonical_y_equals_two.to_vec()).unwrap();

    assert_eq!(
        identity.fingerprint_public_key(&key),
        Err(PortError::IdentityRejected)
    );
}

#[test]
fn rejects_small_order_public_keys() {
    let identity = provider();
    let small_order_key = PeerPublicKey::new(vec![0; 32]).unwrap();

    assert_eq!(
        identity.fingerprint_public_key(&small_order_key),
        Err(PortError::IdentityRejected)
    );
    assert_eq!(
        identity.verify_signature(&small_order_key, b"", &[0; 64]),
        Err(PortError::IdentityRejected)
    );
}
