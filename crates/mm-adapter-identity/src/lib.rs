use curve25519_dalek::edwards::CompressedEdwardsY;
use ed25519_dalek::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, VerifyingKey};
use mm_domain::{PeerFingerprint, PeerPublicKey};
use mm_ports::{PairingIdentityPort, PortError};
use sha2::{Digest, Sha256};

/// Stateless verifier for the proposed raw Ed25519 pairing identity encoding.
///
/// Private-key creation and signing stay behind a separate platform identity
/// adapter. This type accepts only public key bytes and signatures.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ed25519PairingIdentity;

impl PairingIdentityPort for Ed25519PairingIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<PeerFingerprint, PortError> {
        let key = parse_public_key(public_key)?;
        let digest = Sha256::digest(key.to_bytes());
        let fingerprint: [u8; 32] = digest.into();
        Ok(PeerFingerprint::from_bytes(fingerprint))
    }

    fn verify_signature(
        &self,
        public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        if signature.len() != SIGNATURE_LENGTH {
            return Err(PortError::IdentityRejected);
        }

        let key = parse_public_key(public_key)?;
        let signature = Signature::try_from(signature).map_err(|_| PortError::IdentityRejected)?;
        key.verify_strict(message, &signature)
            .map_err(|_| PortError::IdentityRejected)
    }
}

fn parse_public_key(public_key: &PeerPublicKey) -> Result<VerifyingKey, PortError> {
    let bytes: &[u8; PUBLIC_KEY_LENGTH] = public_key
        .as_bytes()
        .try_into()
        .map_err(|_| PortError::IdentityRejected)?;
    let point = CompressedEdwardsY(*bytes)
        .decompress()
        .ok_or(PortError::IdentityRejected)?;
    if point.compress().to_bytes() != *bytes || point.is_small_order() {
        return Err(PortError::IdentityRejected);
    }
    VerifyingKey::from_bytes(bytes).map_err(|_| PortError::IdentityRejected)
}
