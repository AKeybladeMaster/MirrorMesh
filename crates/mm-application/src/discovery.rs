use std::collections::HashMap;

use mm_domain::DeviceId;
use mm_ports::{DiscoveryCandidate, DiscoveryCandidatesPort, PortError};

/// Application boundary for listing nearby devices as unauthenticated pairing candidates.
///
/// Candidates that claim the local ID, have no usable endpoint, or duplicate another claimed
/// ID in the same scan are omitted. No identity verification or trust persistence happens here.
pub struct DiscoveryCoordinator<'a, D: DiscoveryCandidatesPort> {
    local_device_id: DeviceId,
    discovery: &'a D,
}

impl<'a, D: DiscoveryCandidatesPort> DiscoveryCoordinator<'a, D> {
    pub fn new(local_device_id: DeviceId, discovery: &'a D) -> Self {
        Self {
            local_device_id,
            discovery,
        }
    }

    /// Returns a stable device-ID ordered list of unambiguous, endpoint-bearing hints.
    pub fn discover(&self) -> Result<Vec<DiscoveryCandidate>, PortError> {
        let candidates = self.discovery.discover_candidates()?;
        let mut counts = HashMap::new();
        for candidate in &candidates {
            if candidate.device.id != self.local_device_id && !candidate.endpoints.is_empty() {
                *counts.entry(candidate.device.id).or_insert(0usize) += 1;
            }
        }

        let mut candidates: Vec<_> = candidates
            .into_iter()
            .filter(|candidate| {
                candidate.device.id != self.local_device_id
                    && !candidate.endpoints.is_empty()
                    && counts.get(&candidate.device.id) == Some(&1)
            })
            .collect();
        for candidate in &mut candidates {
            candidate.endpoints.sort();
            candidate.endpoints.dedup();
        }
        candidates.sort_by_key(|candidate| candidate.device.id);
        Ok(candidates)
    }

    /// Cancels an active adapter scan when supported by the platform implementation.
    pub fn stop(&self) -> Result<(), PortError> {
        self.discovery.stop()
    }
}
