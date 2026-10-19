use std::fmt;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mm_adapter_signaling::{
    PairingConnection, PendingPairingConnection, WssApprovalError, WssControlChannelError,
    WssListenerError, WssPairingListener,
};
use mm_adapter_tls::{TlsAdapterError, tls13_server_config};
use mm_application::PairingInvitation;
use mm_domain::{DeviceId, PeerFingerprint, TrustedPeer, TrustedPeerGrants};
use mm_ports::{
    LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError, TrustedPeerPort,
};
use mm_runtime::{PairingRuntime, PairingRuntimeError};

/// Android TV receiver service over the shared authenticated pairing runtime and WSS adapter.
///
/// Call [`Self::bind`] only after an explicit user action. Calls to [`Self::accept_request`] block
/// while waiting for the next request and must run on a worker thread. [`Self::cancel_wait`] ends
/// only the current wait so the caller can refresh a one-time invitation; [`Self::stop_accepting`]
/// permanently stops this service instance. The returned pending request contains verified
/// identity metadata only; it is not trusted until [`Self::approve`] succeeds after explicit
/// user consent.
pub struct AndroidTvPairingService<P, I>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort + LocalTlsIdentityPort + 'static,
{
    listener: WssPairingListener,
    runtime: Arc<Mutex<PairingRuntime<P, I>>>,
    cancellation: AtomicBool,
    stopped: AtomicBool,
}

impl<P, I> AndroidTvPairingService<P, I>
where
    P: TrustedPeerPort,
    I: LocalPairingIdentityPort + PairingIdentityPort + LocalTlsIdentityPort + 'static,
{
    /// Binds the receiver to the caller-selected local address using its existing pairing key.
    pub fn bind(
        address: SocketAddr,
        runtime: PairingRuntime<P, I>,
        identity: Arc<I>,
    ) -> Result<Self, AndroidTvPairingServiceError> {
        Self::bind_shared(address, Arc::new(Mutex::new(runtime)), identity)
    }

    /// Binds a listener while retaining the caller's process-owned pairing runtime.
    pub fn bind_shared(
        address: SocketAddr,
        runtime: Arc<Mutex<PairingRuntime<P, I>>>,
        identity: Arc<I>,
    ) -> Result<Self, AndroidTvPairingServiceError> {
        let runtime_fingerprint = runtime
            .lock()
            .map_err(|_| AndroidTvPairingServiceError::RuntimeLockPoisoned)?
            .local_fingerprint();
        let pairing_public_key = identity
            .local_public_key()
            .map_err(AndroidTvPairingServiceError::Platform)?;
        let tls_public_key = identity
            .tls_server_public_key()
            .map_err(AndroidTvPairingServiceError::Platform)?;
        let tls_fingerprint = identity
            .fingerprint_public_key(&tls_public_key)
            .map_err(AndroidTvPairingServiceError::Platform)?;
        if pairing_public_key != tls_public_key || tls_fingerprint != runtime_fingerprint {
            return Err(AndroidTvPairingServiceError::IdentityMismatch);
        }

        let tls_identity: Arc<dyn LocalTlsIdentityPort> = identity;
        let tls_config =
            tls13_server_config(tls_identity).map_err(AndroidTvPairingServiceError::Tls)?;
        let listener = WssPairingListener::bind(address, tls_config)
            .map_err(WssListenerError::Io)
            .map_err(AndroidTvPairingServiceError::Listener)?;
        Ok(Self {
            listener,
            runtime,
            cancellation: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
        })
    }

    /// Returns the actual bound address, including an OS-assigned port when bound to port zero.
    pub fn local_addr(&self) -> Result<SocketAddr, AndroidTvPairingServiceError> {
        self.listener
            .local_addr()
            .map_err(WssListenerError::Io)
            .map_err(AndroidTvPairingServiceError::Listener)
    }

    /// Creates a short-lived invitation for the explicitly selected numeric LAN endpoint.
    pub fn create_invitation(
        &self,
        endpoint: &str,
        now_monotonic_ms: u64,
        now_unix_seconds: u64,
    ) -> Result<PairingInvitation, AndroidTvPairingServiceError> {
        let port = self.local_addr()?.port();
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| AndroidTvPairingServiceError::RuntimeLockPoisoned)?;
        runtime
            .create_invitation(endpoint, &[port], now_monotonic_ms, now_unix_seconds)
            .map_err(|error| {
                AndroidTvPairingServiceError::Pairing(PairingRuntimeError::Coordinator(error))
            })
    }

    /// Accepts one bounded WSS request and verifies its signed proof without saving trust.
    pub fn accept_request(
        &self,
        now_monotonic_ms: u64,
    ) -> Result<PendingAndroidTvPairing, AndroidTvPairingServiceError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(AndroidTvPairingServiceError::Listener(
                WssListenerError::Cancelled,
            ));
        }
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| AndroidTvPairingServiceError::RuntimeLockPoisoned)?;
        let result = self
            .listener
            .accept_pairing_request_cancellable(&mut runtime, now_monotonic_ms, &self.cancellation)
            .map_err(AndroidTvPairingServiceError::Listener);
        if !self.stopped.load(Ordering::Acquire) {
            self.cancellation.store(false, Ordering::Release);
        }
        let pending = result?;
        let approval = pending.admitted().approval();
        let request = AndroidTvPairingRequest {
            source_ip: pending.admitted().source_ip(),
            peer_id: approval.peer_id(),
            display_name: approval.display_name().to_owned(),
            fingerprint: approval.fingerprint(),
            requested_grants: approval.requested_grants(),
        };
        Ok(PendingAndroidTvPairing { request, pending })
    }

    /// Cancels an idle accept loop. Once stopped, this service instance cannot accept again.
    pub fn stop_accepting(&self) {
        self.stopped.store(true, Ordering::Release);
        self.cancellation.store(true, Ordering::Release);
    }

    /// Cancels the current idle wait while leaving the listener available for another wait.
    pub fn cancel_wait(&self) {
        if !self.stopped.load(Ordering::Acquire) {
            self.cancellation.store(true, Ordering::Release);
        }
    }

    /// Returns whether the listener is still accepting requests.
    pub fn is_accepting(&self) -> bool {
        !self.stopped.load(Ordering::Acquire)
    }

    /// Persists only an explicitly accepted subset and returns its approved control channel.
    pub fn approve(
        &self,
        pending: PendingAndroidTvPairing,
        approved_grants: TrustedPeerGrants,
        now_monotonic_ms: u64,
        paired_at_unix_ms: u64,
    ) -> Result<(TrustedPeer, PairingConnection), AndroidTvPairingServiceError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| AndroidTvPairingServiceError::RuntimeLockPoisoned)?;
        pending
            .pending
            .approve(
                &runtime,
                approved_grants,
                now_monotonic_ms,
                paired_at_unix_ms,
            )
            .map_err(AndroidTvPairingServiceError::Approval)
    }
}

/// Display-safe metadata for a verified request awaiting receiver consent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidTvPairingRequest {
    pub source_ip: std::net::IpAddr,
    pub peer_id: DeviceId,
    pub display_name: String,
    pub fingerprint: PeerFingerprint,
    pub requested_grants: TrustedPeerGrants,
}

/// A verified sender request that still requires explicit Android TV user consent.
pub struct PendingAndroidTvPairing {
    request: AndroidTvPairingRequest,
    pending: PendingPairingConnection,
}

impl PendingAndroidTvPairing {
    pub fn request(&self) -> &AndroidTvPairingRequest {
        &self.request
    }

    /// Sends an explicit rejection and closes without persisting a trusted peer.
    pub fn reject(
        self,
        reason: &str,
        sent_at_unix_ms: u64,
    ) -> Result<(), AndroidTvPairingServiceError> {
        self.pending
            .reject(reason, sent_at_unix_ms)
            .map_err(AndroidTvPairingServiceError::Control)
    }
}

#[derive(Debug)]
pub enum AndroidTvPairingServiceError {
    Platform(PortError),
    Tls(TlsAdapterError),
    Listener(WssListenerError),
    Pairing(PairingRuntimeError),
    Approval(WssApprovalError),
    Control(WssControlChannelError),
    IdentityMismatch,
    RuntimeLockPoisoned,
}

impl fmt::Display for AndroidTvPairingServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(error) => {
                write!(formatter, "Android TV platform runtime failed: {error}")
            }
            Self::Tls(error) => write!(formatter, "Android TV TLS identity failed: {error}"),
            Self::Listener(error) => write!(formatter, "Android TV WSS listener failed: {error}"),
            Self::Pairing(error) => write!(formatter, "Android TV pairing failed: {error}"),
            Self::Approval(error) => write!(formatter, "Android TV approval failed: {error}"),
            Self::Control(error) => write!(formatter, "Android TV control channel failed: {error}"),
            Self::IdentityMismatch => {
                formatter.write_str("Android TV pairing and TLS identities do not match")
            }
            Self::RuntimeLockPoisoned => {
                formatter.write_str("Android TV pairing runtime lock is unavailable")
            }
        }
    }
}

impl std::error::Error for AndroidTvPairingServiceError {}

#[cfg(test)]
mod tests;
