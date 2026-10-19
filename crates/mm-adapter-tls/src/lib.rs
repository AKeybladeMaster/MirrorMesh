//! TLS 1.3 raw-public-key authentication pinned to the device fingerprint in its pairing QR.

use std::fmt;
use std::sync::Arc;

use mm_domain::{PeerFingerprint, PeerPublicKey};
use mm_ports::{LocalTlsIdentityPort, PortError};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, SubjectPublicKeyInfoDer, UnixTime};
use rustls::server::AlwaysResolvesServerRawPublicKeys;
use rustls::sign::{CertifiedKey, Signer, SigningKey};
use rustls::{
    ClientConfig, DigitallySignedStruct, Error as RustlsError, ServerConfig, SignatureAlgorithm,
    SignatureScheme,
};
use sha2::{Digest, Sha256};

const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];
const ED25519_PUBLIC_KEY_BYTES: usize = 32;
const ED25519_SIGNATURE_BYTES: usize = 64;
const TLS13_SERVER_CERT_VERIFY_CONTEXT: &[u8; 34] = b"TLS 1.3, server CertificateVerify\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsAdapterError {
    Identity(PortError),
    UnsupportedPublicKey,
    RustlsConfiguration,
}

impl fmt::Display for TlsAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => write!(formatter, "TLS identity is unavailable: {error}"),
            Self::UnsupportedPublicKey => {
                formatter.write_str("TLS identity must be a 32-byte Ed25519 public key")
            }
            Self::RustlsConfiguration => formatter.write_str("TLS 1.3 configuration failed"),
        }
    }
}

impl std::error::Error for TlsAdapterError {}

/// Creates a native TLS 1.3 server config that presents the local identity as an RFC 7250 raw
/// public key. The identity adapter retains the private key and signs only through the TLS port.
pub fn tls13_server_config(
    identity: Arc<dyn LocalTlsIdentityPort>,
) -> Result<Arc<ServerConfig>, TlsAdapterError> {
    let public_key = identity
        .tls_server_public_key()
        .map_err(TlsAdapterError::Identity)?;
    let spki = ed25519_spki_der(&public_key)?;
    let signing_key = Arc::new(PlatformEd25519SigningKey {
        identity,
        spki: spki.clone(),
    });
    let certified_key = Arc::new(CertifiedKey::new(
        vec![CertificateDer::from(spki)],
        signing_key,
    ));
    // `CertifiedKey::keys_match()` parses an X.509 certificate. RFC 7250 places a bare SPKI in
    // this field, so the client verifies key possession through TLS 1.3 CertificateVerify.

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| TlsAdapterError::RustlsConfiguration)?;
    let mut config = builder.with_no_client_auth().with_cert_resolver(Arc::new(
        AlwaysResolvesServerRawPublicKeys::new(certified_key),
    ));

    // A resumed session could otherwise keep authenticating an identity after its key rotates.
    config.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
    config.send_tls13_tickets = 0;
    config.max_early_data_size = 0;
    Ok(Arc::new(config))
}

/// Creates a native TLS 1.3 client config that accepts only the raw Ed25519 key whose SHA-256
/// fingerprint is encoded in the validated pairing QR.
pub fn tls13_pinned_client_config(
    expected_fingerprint: PeerFingerprint,
) -> Result<Arc<ClientConfig>, TlsAdapterError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = Arc::new(PinnedEd25519ServerVerifier {
        expected_fingerprint,
        provider: Arc::clone(&provider),
    });
    let builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| TlsAdapterError::RustlsConfiguration)?;
    let mut config = builder
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    config.resumption = rustls::client::Resumption::disabled();
    Ok(Arc::new(config))
}

fn ed25519_spki_der(public_key: &PeerPublicKey) -> Result<Vec<u8>, TlsAdapterError> {
    if public_key.as_bytes().len() != ED25519_PUBLIC_KEY_BYTES {
        return Err(TlsAdapterError::UnsupportedPublicKey);
    }

    let mut spki = Vec::with_capacity(ED25519_SPKI_PREFIX.len() + ED25519_PUBLIC_KEY_BYTES);
    spki.extend_from_slice(&ED25519_SPKI_PREFIX);
    spki.extend_from_slice(public_key.as_bytes());
    Ok(spki)
}

fn ed25519_public_key_from_spki(spki: &[u8]) -> Option<&[u8]> {
    let (prefix, public_key) = spki.split_at_checked(ED25519_SPKI_PREFIX.len())?;
    (prefix == ED25519_SPKI_PREFIX && public_key.len() == ED25519_PUBLIC_KEY_BYTES)
        .then_some(public_key)
}

fn is_tls13_server_certificate_verify_input(message: &[u8]) -> bool {
    const TLS13_SIGNATURE_INPUT_PREFIX_BYTES: usize = 64 + TLS13_SERVER_CERT_VERIFY_CONTEXT.len();
    const SHA256_TRANSCRIPT_HASH_BYTES: usize = 32;
    const SHA384_TRANSCRIPT_HASH_BYTES: usize = 48;

    (message.len() == TLS13_SIGNATURE_INPUT_PREFIX_BYTES + SHA256_TRANSCRIPT_HASH_BYTES
        || message.len() == TLS13_SIGNATURE_INPUT_PREFIX_BYTES + SHA384_TRANSCRIPT_HASH_BYTES)
        && message[..64].iter().all(|byte| *byte == 0x20)
        && message[64..TLS13_SIGNATURE_INPUT_PREFIX_BYTES] == TLS13_SERVER_CERT_VERIFY_CONTEXT[..]
}

struct PinnedEd25519ServerVerifier {
    expected_fingerprint: PeerFingerprint,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl fmt::Debug for PinnedEd25519ServerVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedEd25519ServerVerifier")
            .finish_non_exhaustive()
    }
}

impl ServerCertVerifier for PinnedEd25519ServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        if !intermediates.is_empty() {
            return Err(identity_rejected());
        }
        let public_key =
            ed25519_public_key_from_spki(end_entity.as_ref()).ok_or_else(identity_rejected)?;
        let fingerprint: [u8; 32] = Sha256::digest(public_key).into();
        if fingerprint != *self.expected_fingerprint.as_bytes() {
            return Err(identity_rejected());
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Err(identity_rejected())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        let spki = SubjectPublicKeyInfoDer::from(cert.as_ref());
        rustls::crypto::verify_tls13_signature_with_raw_key(
            message,
            &spki,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }

    fn requires_raw_public_keys(&self) -> bool {
        true
    }
}

struct PlatformEd25519SigningKey {
    identity: Arc<dyn LocalTlsIdentityPort>,
    spki: Vec<u8>,
}

impl fmt::Debug for PlatformEd25519SigningKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformEd25519SigningKey")
            .finish_non_exhaustive()
    }
}

impl SigningKey for PlatformEd25519SigningKey {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn Signer>> {
        offered
            .contains(&SignatureScheme::ED25519)
            .then(|| Box::new(PlatformEd25519Signer(Arc::clone(&self.identity))) as Box<dyn Signer>)
    }

    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::ED25519
    }

    fn public_key(&self) -> Option<SubjectPublicKeyInfoDer<'_>> {
        Some(SubjectPublicKeyInfoDer::from(self.spki.as_slice()))
    }
}

struct PlatformEd25519Signer(Arc<dyn LocalTlsIdentityPort>);

impl fmt::Debug for PlatformEd25519Signer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformEd25519Signer")
            .finish_non_exhaustive()
    }
}

impl Signer for PlatformEd25519Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, RustlsError> {
        if !is_tls13_server_certificate_verify_input(message) {
            return Err(identity_rejected());
        }
        let signature = self
            .0
            .sign_tls13_certificate_verify(message)
            .map_err(|_| identity_rejected())?;
        if signature.len() != ED25519_SIGNATURE_BYTES {
            return Err(identity_rejected());
        }
        Ok(signature)
    }

    fn scheme(&self) -> SignatureScheme {
        SignatureScheme::ED25519
    }
}

fn identity_rejected() -> RustlsError {
    RustlsError::General("MirrorMesh TLS identity verification failed".to_owned())
}
