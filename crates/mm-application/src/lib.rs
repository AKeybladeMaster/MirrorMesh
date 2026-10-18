mod pairing;

pub use pairing::{
    MAX_PAIRING_SIGNATURE_BYTES, PairingCoordinator, PairingCoordinatorError, PairingInvitation,
    PairingProof, PendingPairingApproval,
};

#[cfg(test)]
mod tests;
