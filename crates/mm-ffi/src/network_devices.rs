use mm_adapter_discovery::MdnsDiscoveryAdapter;
use serde_json::json;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
static RECEIVER_ADVERTISEMENT: std::sync::OnceLock<mm_adapter_discovery::MdnsServicePublisher> =
    std::sync::OnceLock::new();

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub(crate) fn publish_receiver(
    id: mm_domain::DeviceId,
    address: std::net::SocketAddr,
    fingerprint: mm_domain::PeerFingerprint,
) -> Result<(), mm_ports::PortError> {
    use mm_domain::{CapabilitySet, Device, DeviceRole};
    let capabilities = CapabilitySet::new(1920, 1080, 60, true, false)
        .map_err(|_| mm_ports::PortError::InvalidConfiguration)?;
    let short_id = id.to_string()[..8].to_owned();
    let device = Device::new(
        id,
        format!("MirrorMesh PC {short_id}"),
        DeviceRole::receiver(),
        capabilities,
    )
    .map_err(|_| mm_ports::PortError::InvalidConfiguration)?;
    RECEIVER_ADVERTISEMENT
        .get_or_init(Default::default)
        .publish_with_fingerprint(
            &device,
            &format!("mirrormesh-{short_id}.local."),
            &[address.ip()],
            Some(fingerprint),
        )
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub(crate) fn stop_advertisement() {
    if let Some(publisher) = RECEIVER_ADVERTISEMENT.get() {
        let _ = publisher.stop();
    }
}
use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Runs the existing bounded mDNS browser. Call on a worker, after the user's Search action.
/// Returns owned UTF-8 JSON metadata; release it exactly once with the companion free function.
#[unsafe(no_mangle)]
pub extern "C" fn mirrormesh_discover_devices() -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| {
        match MdnsDiscoveryAdapter::new().discover_services() {
            Ok(candidates) => json!({"ok":true, "devices": candidates.into_iter().map(|candidate| {
                json!({"id":candidate.device.id.to_string(), "name":candidate.device.display_name,
                    "send":candidate.device.role.can_send(), "receive":candidate.device.role.can_receive(),
                    "request_fingerprint":candidate.request_fingerprint.map(|fp| fp.to_hex()),
                    "endpoints":candidate.endpoint.addresses.iter().filter_map(|address| {
                        format!("{}:{}", address, candidate.endpoint.port).parse::<std::net::SocketAddr>().ok()
                            .or_else(|| format!("[{}]:{}", address, candidate.endpoint.port).parse().ok())
                    }).map(|address| address.to_string()).collect::<Vec<_>>()})
            }).collect::<Vec<_>>() }),
            Err(_) => json!({"ok":false,"error":"Local device search is unavailable. Check multicast and app-specific firewall access."}),
        }
    })).unwrap_or_else(|_| json!({"ok":false,"error":"Local device search could not complete."}));
    CString::new(result.to_string())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// `value` must be null or an unreleased pointer returned by this library's JSON functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mirrormesh_free_json(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: Ownership is returned once by the caller under the function contract.
        drop(unsafe { CString::from_raw(value) });
    }
}
