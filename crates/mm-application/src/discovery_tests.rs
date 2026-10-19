use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};

use mm_domain::{CapabilitySet, Device, DeviceId, DeviceRole};
use mm_ports::{DiscoveryCandidate, DiscoveryCandidatesPort, DiscoveryPort, PortError};

use super::DiscoveryCoordinator;

const LOCAL_ID: DeviceId = DeviceId::from_bytes([1; 16]);

struct FakeDiscovery {
    candidates: Vec<DiscoveryCandidate>,
    failure: bool,
    stopped: AtomicBool,
}

impl DiscoveryPort for FakeDiscovery {
    fn discover(&self) -> Result<Vec<Device>, PortError> {
        if self.failure {
            Err(PortError::Unavailable)
        } else {
            Ok(self
                .candidates
                .iter()
                .map(|candidate| candidate.device.clone())
                .collect())
        }
    }

    fn stop(&self) -> Result<(), PortError> {
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl DiscoveryCandidatesPort for FakeDiscovery {
    fn discover_candidates(&self) -> Result<Vec<DiscoveryCandidate>, PortError> {
        if self.failure {
            Err(PortError::Unavailable)
        } else {
            Ok(self.candidates.clone())
        }
    }
}

fn candidate(id: DeviceId, name: &str, port: u16) -> DiscoveryCandidate {
    DiscoveryCandidate {
        device: Device::new(
            id,
            name.to_owned(),
            DeviceRole::receiver(),
            CapabilitySet::new(1920, 1080, 30, true, false).unwrap(),
        )
        .unwrap(),
        endpoints: vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, port as u8)),
            8765,
        )],
    }
}

#[test]
fn discovery_filters_the_local_device_and_returns_stable_order() {
    let discovery = FakeDiscovery {
        candidates: vec![
            candidate(DeviceId::from_bytes([3; 16]), "Third", 3),
            candidate(LOCAL_ID, "This device", 1),
            candidate(DeviceId::from_bytes([2; 16]), "Second", 2),
        ],
        failure: false,
        stopped: AtomicBool::new(false),
    };
    let coordinator = DiscoveryCoordinator::new(LOCAL_ID, &discovery);

    let discovered = coordinator.discover().unwrap();

    assert_eq!(
        discovered
            .iter()
            .map(|entry| entry.device.id)
            .collect::<Vec<_>>(),
        vec![DeviceId::from_bytes([2; 16]), DeviceId::from_bytes([3; 16])]
    );
    assert_eq!(discovered[0].endpoints[0].port(), 8765);
}

#[test]
fn discovery_drops_endpointless_and_ambiguous_duplicate_identity_hints() {
    let mut endpointless = candidate(DeviceId::from_bytes([4; 16]), "No endpoint", 4);
    endpointless.endpoints.clear();
    let discovery = FakeDiscovery {
        candidates: vec![
            candidate(DeviceId::from_bytes([2; 16]), "First claim", 2),
            candidate(DeviceId::from_bytes([2; 16]), "Conflicting claim", 3),
            endpointless,
            candidate(DeviceId::from_bytes([5; 16]), "Usable", 5),
        ],
        failure: false,
        stopped: AtomicBool::new(false),
    };
    let coordinator = DiscoveryCoordinator::new(LOCAL_ID, &discovery);

    let discovered = coordinator.discover().unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].device.display_name, "Usable");
}

#[test]
fn discovery_propagates_adapter_errors_and_exposes_stop() {
    let failing = FakeDiscovery {
        candidates: Vec::new(),
        failure: true,
        stopped: AtomicBool::new(false),
    };
    let coordinator = DiscoveryCoordinator::new(LOCAL_ID, &failing);

    assert_eq!(coordinator.discover(), Err(PortError::Unavailable));
    assert_eq!(coordinator.stop(), Ok(()));
    assert!(failing.stopped.load(Ordering::SeqCst));
}
