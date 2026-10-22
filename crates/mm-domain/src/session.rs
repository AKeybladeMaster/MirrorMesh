use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Discovered,
    Pairing,
    Paired,
    Negotiating,
    Streaming,
    Reconnecting,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    PeerDiscovered,
    BeginPairing,
    PairingApproved {
        fingerprint_verified: bool,
        user_approved: bool,
    },
    TrustedPeerVerified {
        fingerprint_verified: bool,
    },
    BeginNegotiation,
    StartStreaming {
        peer_authenticated: bool,
        capture_permitted: bool,
        first_frame_presented: bool,
    },
    ConnectionLost,
    ReconnectSucceeded {
        peer_authenticated: bool,
        capture_permitted: bool,
        first_frame_presented: bool,
    },
    ReconnectBudgetExhausted,
    Stop,
    CleanupComplete,
    Reset,
}

pub fn try_transition(
    current: SessionState,
    event: SessionEvent,
) -> Result<SessionState, DomainError> {
    use SessionEvent as E;
    use SessionState as S;

    let next = match (current, event) {
        (S::Idle, E::PeerDiscovered) => S::Discovered,
        (S::Stopped, E::Reset) => S::Idle,
        (S::Discovered, E::BeginPairing) => S::Pairing,
        (
            S::Pairing,
            E::PairingApproved {
                fingerprint_verified: true,
                user_approved: true,
            },
        ) => S::Paired,
        (S::Pairing, E::PairingApproved { .. }) => return Err(DomainError::AuthorizationDenied),
        (
            S::Discovered,
            E::TrustedPeerVerified {
                fingerprint_verified: true,
            },
        ) => S::Paired,
        (
            S::Discovered,
            E::TrustedPeerVerified {
                fingerprint_verified: false,
            },
        ) => {
            return Err(DomainError::AuthorizationDenied);
        }
        (S::Paired, E::BeginNegotiation) => S::Negotiating,
        (
            S::Negotiating,
            E::StartStreaming {
                peer_authenticated: true,
                capture_permitted: true,
                first_frame_presented: true,
            },
        ) => S::Streaming,
        (S::Negotiating, E::StartStreaming { .. }) => return Err(DomainError::AuthorizationDenied),
        (S::Streaming, E::ConnectionLost) => S::Reconnecting,
        (
            S::Reconnecting,
            E::ReconnectSucceeded {
                peer_authenticated: true,
                capture_permitted: true,
                first_frame_presented: true,
            },
        ) => S::Streaming,
        (S::Reconnecting, E::ReconnectSucceeded { .. }) => {
            return Err(DomainError::AuthorizationDenied);
        }
        (S::Reconnecting, E::ReconnectBudgetExhausted) => S::Failed,
        (S::Idle | S::Stopped, E::Stop) => current,
        (S::Stopping, E::Stop) => S::Stopping,
        (S::Failed, E::Stop) => S::Stopping,
        (_, E::Stop) => S::Stopping,
        (S::Stopping | S::Failed, E::CleanupComplete) => S::Stopped,
        _ => return Err(DomainError::IllegalTransition),
    };
    Ok(next)
}
