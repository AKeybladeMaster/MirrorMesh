mod discovery;
mod pairing;

pub use discovery::DiscoveryCoordinator;

#[cfg(test)]
mod discovery_tests;

pub use pairing::{
    MAX_PAIRING_SIGNATURE_BYTES, PairingCoordinator, PairingCoordinatorError, PairingInvitation,
    PairingProof, PendingPairingApproval,
};

#[cfg(test)]
mod tests;
