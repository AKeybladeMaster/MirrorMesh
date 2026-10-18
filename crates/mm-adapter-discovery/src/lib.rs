use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use mdns_sd::{ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent};
use mm_domain::{CapabilitySet, Device, DeviceId, DeviceRole};
use mm_ports::{DiscoveryPort, PortError};

pub const MIRRORMESH_SERVICE_TYPE: &str = "_mirrormesh._tcp.local.";
pub const MIRRORMESH_SIGNALING_PORT: u16 = 8765;
const MIRRORMESH_INSTANCE_SUFFIX: &str = "._mirrormesh._tcp.local.";

const DISCOVERY_WINDOW: Duration = Duration::from_secs(2);
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_ENDPOINT_ADDRESSES: usize = 16;
const MAX_DISCOVERED_SERVICES: usize = 64;
const TXT_KEYS: [&str; 8] = ["v", "id", "roles", "pair", "tls", "video", "audio", "max"];

/// A resolved address and signaling port from an unauthenticated mDNS response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredEndpoint {
    pub port: u16,
    pub addresses: Vec<ScopedIp>,
}

/// An unauthenticated discovery hint. Pairing must still verify identity and require approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredService {
    pub device: Device,
    pub endpoint: DiscoveredEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvertisementError {
    InvalidServiceType,
    InvalidRecord,
    InvalidEndpoint,
}

/// Parses one resolved service. Advertisements are public, untrusted discovery hints only.
pub fn parse_advertisement(
    service_type: &str,
    fullname: &str,
    port: u16,
    addresses: Vec<ScopedIp>,
    properties: &HashMap<String, String>,
) -> Result<DiscoveredService, AdvertisementError> {
    if !service_type.eq_ignore_ascii_case(MIRRORMESH_SERVICE_TYPE) {
        return Err(AdvertisementError::InvalidServiceType);
    }

    let Some(name_end) = fullname.len().checked_sub(MIRRORMESH_INSTANCE_SUFFIX.len()) else {
        return Err(AdvertisementError::InvalidServiceType);
    };
    if !fullname.is_char_boundary(name_end)
        || !fullname[name_end..].eq_ignore_ascii_case(MIRRORMESH_INSTANCE_SUFFIX)
    {
        return Err(AdvertisementError::InvalidServiceType);
    }
    let display_name = &fullname[..name_end];
    if display_name.is_empty() || display_name.len() > 63 || port != MIRRORMESH_SIGNALING_PORT {
        return Err(AdvertisementError::InvalidEndpoint);
    }

    let mut normalized_properties = HashMap::with_capacity(properties.len());
    for (key, property_value) in properties {
        if normalized_properties
            .insert(key.to_ascii_lowercase(), property_value.as_str())
            .is_some()
        {
            return Err(AdvertisementError::InvalidRecord);
        }
    }
    if normalized_properties.len() != TXT_KEYS.len()
        || normalized_properties
            .keys()
            .any(|key| !TXT_KEYS.contains(&key.as_str()))
    {
        return Err(AdvertisementError::InvalidRecord);
    }

    let value = |key: &str| {
        normalized_properties
            .get(key)
            .copied()
            .ok_or(AdvertisementError::InvalidRecord)
    };
    if value("v")? != "1" || value("pair")? != "qr" || value("tls")? != "1" {
        return Err(AdvertisementError::InvalidRecord);
    }

    let id = DeviceId::parse(value("id")?).map_err(|_| AdvertisementError::InvalidRecord)?;
    let role = parse_roles(value("roles")?)?;
    let capabilities = parse_capabilities(value("video")?, value("audio")?, value("max")?)?;
    let device = Device::new(id, display_name.to_owned(), role, capabilities)
        .map_err(|_| AdvertisementError::InvalidRecord)?;

    let mut usable_addresses: Vec<_> = addresses.into_iter().filter(is_usable_address).collect();
    if usable_addresses.is_empty() {
        return Err(AdvertisementError::InvalidEndpoint);
    }
    usable_addresses.sort_by_key(ToString::to_string);
    usable_addresses.truncate(MAX_ENDPOINT_ADDRESSES);

    Ok(DiscoveredService {
        device,
        endpoint: DiscoveredEndpoint {
            port,
            addresses: usable_addresses,
        },
    })
}

fn parse_roles(value: &str) -> Result<DeviceRole, AdvertisementError> {
    match value {
        "send" => Ok(DeviceRole::sender()),
        "recv" => Ok(DeviceRole::receiver()),
        "send,recv" => Ok(DeviceRole::sender_and_receiver()),
        _ => Err(AdvertisementError::InvalidRecord),
    }
}

fn parse_capabilities(
    video: &str,
    audio: &str,
    maximum: &str,
) -> Result<CapabilitySet, AdvertisementError> {
    if video != "h264" {
        return Err(AdvertisementError::InvalidRecord);
    }
    let audio = match audio {
        "opus" => true,
        "none" => false,
        _ => return Err(AdvertisementError::InvalidRecord),
    };
    let (dimensions, fps) = maximum
        .split_once('@')
        .ok_or(AdvertisementError::InvalidRecord)?;
    let (width, height) = dimensions
        .split_once('x')
        .ok_or(AdvertisementError::InvalidRecord)?;
    if fps.contains('@') || height.contains('x') {
        return Err(AdvertisementError::InvalidRecord);
    }
    let width = width
        .parse::<u32>()
        .map_err(|_| AdvertisementError::InvalidRecord)?;
    let height = height
        .parse::<u32>()
        .map_err(|_| AdvertisementError::InvalidRecord)?;
    let fps = fps
        .parse::<u16>()
        .map_err(|_| AdvertisementError::InvalidRecord)?;
    CapabilitySet::new(width, height, fps, true, audio)
        .map_err(|_| AdvertisementError::InvalidRecord)
}

fn is_usable_address(address: &ScopedIp) -> bool {
    match address.to_ip_addr() {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !address.is_broadcast()
        }
        IpAddr::V6(address) => {
            !address.is_unspecified() && !address.is_loopback() && !address.is_multicast()
        }
    }
}

/// Cross-platform mDNS browser for MirrorMesh DNS-SD advertisements.
pub struct MdnsDiscoveryAdapter {
    browse_guard: Mutex<()>,
    active_daemon: Mutex<Option<ServiceDaemon>>,
    cancelled: AtomicBool,
}

impl MdnsDiscoveryAdapter {
    pub fn new() -> Self {
        Self {
            browse_guard: Mutex::new(()),
            active_daemon: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        }
    }

    /// Browses for at most two seconds and returns only structurally valid, untrusted hints.
    pub fn discover_services(&self) -> Result<Vec<DiscoveredService>, PortError> {
        let _browse_guard = self
            .browse_guard
            .lock()
            .map_err(|_| PortError::Unavailable)?;
        self.cancelled.store(false, Ordering::Release);

        let daemon = ServiceDaemon::new().map_err(|_| PortError::Unavailable)?;
        let receiver = match daemon.browse(MIRRORMESH_SERVICE_TYPE) {
            Ok(receiver) => receiver,
            Err(_) => {
                let _ = daemon.shutdown();
                return Err(PortError::Unavailable);
            }
        };

        {
            let mut active = match self.active_daemon.lock() {
                Ok(active) => active,
                Err(_) => {
                    let _ = daemon.shutdown();
                    return Err(PortError::Unavailable);
                }
            };
            *active = Some(daemon.clone());
        }

        let result = self.collect_services(&receiver);
        self.clear_daemon(&daemon);
        result
    }

    fn collect_services(
        &self,
        receiver: &mdns_sd::Receiver<ServiceEvent>,
    ) -> Result<Vec<DiscoveredService>, PortError> {
        let deadline = Instant::now() + DISCOVERY_WINDOW;
        let mut services = BTreeMap::new();

        while !self.cancelled.load(Ordering::Acquire) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match receiver.recv_timeout(remaining.min(EVENT_POLL_INTERVAL)) {
                Ok(ServiceEvent::ServiceResolved(resolved)) => {
                    if let Ok(service) = parse_resolved_service(&resolved) {
                        if services.len() < MAX_DISCOVERED_SERVICES
                            || services.contains_key(&resolved.fullname)
                        {
                            services.insert(resolved.fullname.clone(), service);
                        }
                    }
                }
                Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                    services.remove(&fullname);
                }
                Ok(_) => {}
                Err(_) if self.cancelled.load(Ordering::Acquire) => break,
                Err(_) => return Err(PortError::Unavailable),
            }
        }

        let mut services: Vec<_> = services.into_values().collect();
        services.sort_by_key(|service| service.device.id);
        services.dedup_by_key(|service| service.device.id);
        Ok(services)
    }

    fn clear_daemon(&self, daemon: &ServiceDaemon) {
        if let Ok(mut active) = self.active_daemon.lock() {
            active.take();
        }
        let _ = daemon.stop_browse(MIRRORMESH_SERVICE_TYPE);
        let _ = daemon.shutdown();
    }
}

impl Default for MdnsDiscoveryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryPort for MdnsDiscoveryAdapter {
    fn discover(&self) -> Result<Vec<Device>, PortError> {
        self.discover_services()
            .map(|services| services.into_iter().map(|service| service.device).collect())
    }

    fn stop(&self) -> Result<(), PortError> {
        self.cancelled.store(true, Ordering::Release);
        let daemon = self
            .active_daemon
            .lock()
            .map_err(|_| PortError::Unavailable)?
            .take();
        if let Some(daemon) = daemon {
            daemon.shutdown().map_err(|_| PortError::Unavailable)?;
        }
        Ok(())
    }
}

impl Drop for MdnsDiscoveryAdapter {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Ok(active) = self.active_daemon.get_mut() {
            if let Some(daemon) = active.take() {
                let _ = daemon.shutdown();
            }
        }
    }
}

fn parse_resolved_service(
    resolved: &ResolvedService,
) -> Result<DiscoveredService, AdvertisementError> {
    let properties = resolved.txt_properties.clone().into_property_map_str();
    parse_advertisement(
        &resolved.ty_domain,
        &resolved.fullname,
        resolved.port,
        resolved.addresses.iter().cloned().collect(),
        &properties,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use mdns_sd::ScopedIp;

    use super::{AdvertisementError, DiscoveredService, parse_advertisement};

    const SERVICE_TYPE: &str = "_mirrormesh._tcp.local.";
    const INSTANCE: &str = "Living Room TV._mirrormesh._tcp.local.";

    fn valid_properties() -> HashMap<String, String> {
        HashMap::from([
            ("v".to_owned(), "1".to_owned()),
            (
                "id".to_owned(),
                "8b3d8788-1476-4aae-a287-334a428b5b6f".to_owned(),
            ),
            ("roles".to_owned(), "send,recv".to_owned()),
            ("pair".to_owned(), "qr".to_owned()),
            ("tls".to_owned(), "1".to_owned()),
            ("video".to_owned(), "h264".to_owned()),
            ("audio".to_owned(), "opus".to_owned()),
            ("max".to_owned(), "1920x1080@30".to_owned()),
        ])
    }

    fn ipv4() -> Vec<ScopedIp> {
        vec![ScopedIp::from(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 40)))]
    }

    fn parse(
        service_type: &str,
        fullname: &str,
        port: u16,
        addresses: Vec<ScopedIp>,
        properties: HashMap<String, String>,
    ) -> Result<DiscoveredService, AdvertisementError> {
        parse_advertisement(service_type, fullname, port, addresses, &properties)
    }

    #[test]
    fn maps_a_valid_advertisement_to_an_untrusted_device_hint_and_endpoint() {
        let service = parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), valid_properties()).unwrap();

        assert_eq!(
            service.device.id.to_string(),
            "8b3d8788-1476-4aae-a287-334a428b5b6f"
        );
        assert_eq!(service.device.display_name, "Living Room TV");
        assert!(service.device.role.can_send());
        assert!(service.device.role.can_receive());
        assert_eq!(service.device.capabilities.max_width, 1920);
        assert_eq!(service.device.capabilities.max_height, 1080);
        assert_eq!(service.device.capabilities.max_fps, 30);
        assert!(service.device.capabilities.h264);
        assert!(service.device.capabilities.opus);
        assert_eq!(service.endpoint.port, 8765);
        assert_eq!(service.endpoint.addresses[0].to_string(), "192.168.1.40");
    }

    #[test]
    fn rejects_wrong_service_type_or_mismatched_instance_suffix() {
        let properties = valid_properties();
        assert!(
            parse(
                "_http._tcp.local.",
                INSTANCE,
                8765,
                ipv4(),
                properties.clone()
            )
            .is_err()
        );
        assert!(
            parse(
                SERVICE_TYPE,
                "TV._http._tcp.local.",
                8765,
                ipv4(),
                properties
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_dns_case_insensitive_service_names_and_txt_keys() {
        let properties = valid_properties()
            .into_iter()
            .map(|(key, value)| (key.to_ascii_uppercase(), value))
            .collect();
        let service = parse(
            "_MIRRORMESH._TCP.LOCAL.",
            "Living Room TV._MIRRORMESH._TCP.LOCAL.",
            8765,
            ipv4(),
            properties,
        )
        .unwrap();

        assert_eq!(service.device.display_name, "Living Room TV");
    }

    #[test]
    fn rejects_duplicate_txt_keys_with_different_ascii_case() {
        let mut properties = valid_properties();
        properties.insert("V".to_owned(), "1".to_owned());

        assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());
    }

    #[test]
    fn parses_the_mdns_sd_resolved_service_shape_without_network_io() {
        let properties = [
            ("v", "1"),
            ("id", "8b3d8788-1476-4aae-a287-334a428b5b6f"),
            ("roles", "send,recv"),
            ("pair", "qr"),
            ("tls", "1"),
            ("video", "h264"),
            ("audio", "opus"),
            ("max", "1920x1080@30"),
        ];
        let service_info = mdns_sd::ServiceInfo::new(
            SERVICE_TYPE,
            "Living Room TV",
            "receiver.local.",
            "192.168.1.40",
            8765,
            &properties[..],
        )
        .unwrap();
        let resolved = service_info.as_resolved_service();
        let service = super::parse_resolved_service(&resolved).unwrap();

        assert_eq!(service.device.display_name, "Living Room TV");
        assert_eq!(service.endpoint.port, 8765);
        assert_eq!(service.endpoint.addresses[0].to_string(), "192.168.1.40");
    }

    #[test]
    fn rejects_unknown_or_missing_fields_and_non_v1_records() {
        let mut properties = valid_properties();
        properties.remove("tls");
        assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());

        let mut properties = valid_properties();
        properties.insert("secret".to_owned(), "must-not-be-advertised".to_owned());
        assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());

        let mut properties = valid_properties();
        properties.insert("v".to_owned(), "2".to_owned());
        assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());
    }

    #[test]
    fn rejects_invalid_identity_role_pairing_and_transport_markers() {
        for (key, value) in [
            ("id", "not-a-uuid"),
            ("roles", "send,send"),
            ("roles", "recv,send"),
            ("pair", "pin"),
            ("tls", "0"),
        ] {
            let mut properties = valid_properties();
            properties.insert(key.to_owned(), value.to_owned());
            assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());
        }
    }

    #[test]
    fn rejects_invalid_or_out_of_range_capability_records() {
        for value in ["vp9", "7681x1080@30", "1920x1080@121", "1920x0@30"] {
            let mut properties = valid_properties();
            properties.insert("max".to_owned(), value.to_owned());
            assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());
        }

        let mut properties = valid_properties();
        properties.insert("video".to_owned(), "vp9".to_owned());
        assert!(parse(SERVICE_TYPE, INSTANCE, 8765, ipv4(), properties).is_err());
    }

    #[test]
    fn rejects_unusable_endpoint_data() {
        assert!(parse(SERVICE_TYPE, INSTANCE, 0, ipv4(), valid_properties()).is_err());
        assert!(parse(SERVICE_TYPE, INSTANCE, 8766, ipv4(), valid_properties()).is_err());
        assert!(parse(SERVICE_TYPE, INSTANCE, 8765, Vec::new(), valid_properties()).is_err());
        assert!(
            parse(
                SERVICE_TYPE,
                INSTANCE,
                8765,
                vec![ScopedIp::from(IpAddr::V4(Ipv4Addr::UNSPECIFIED))],
                valid_properties()
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_loopback_ipv6_endpoint() {
        let error = parse(
            SERVICE_TYPE,
            INSTANCE,
            8765,
            vec![ScopedIp::from(IpAddr::V6(Ipv6Addr::LOCALHOST))],
            valid_properties(),
        )
        .unwrap_err();
        assert_eq!(error, AdvertisementError::InvalidEndpoint);
    }

    #[test]
    fn retains_a_usable_ipv6_endpoint() {
        let service = parse(
            SERVICE_TYPE,
            INSTANCE,
            8765,
            vec![ScopedIp::from(IpAddr::V6(
                "2001:4860:4860::8888".parse().unwrap(),
            ))],
            valid_properties(),
        )
        .unwrap();

        assert_eq!(
            service.endpoint.addresses[0].to_string(),
            "2001:4860:4860::8888"
        );
    }
}
