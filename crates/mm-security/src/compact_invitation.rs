//! Compact v2 transport encoding. Authentication remains the pinned TLS handshake.
use super::*;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encodes the full identity, nonce and deadline without a URL shortener or shared service.
pub fn format_pairing_qr(
    endpoint: ManualIpEndpoint,
    device_id: DeviceId,
    fingerprint: PeerFingerprint,
    nonce: PairingNonce,
    expires_at_unix_seconds: u64,
) -> String {
    let address = endpoint.socket_addr();
    let mut bytes = Vec::with_capacity(91);
    match address.ip() {
        IpAddr::V4(ip) => {
            bytes.push(4);
            bytes.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            bytes.push(6);
            bytes.extend_from_slice(&ip.octets());
        }
    }
    bytes.extend_from_slice(&address.port().to_be_bytes());
    bytes.extend_from_slice(device_id.as_bytes());
    bytes.extend_from_slice(fingerprint.as_bytes());
    bytes.extend_from_slice(nonce.as_bytes());
    bytes.extend_from_slice(&expires_at_unix_seconds.to_be_bytes());
    let mut uri = String::from("mirrormesh://p/");
    let mut accumulator = 0_u32;
    let mut bits = 0;
    for byte in bytes {
        accumulator = (accumulator << 8) | u32::from(byte);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            uri.push(ALPHABET[((accumulator >> bits) & 63) as usize] as char);
        }
    }
    if bits != 0 {
        uri.push(ALPHABET[((accumulator << (6 - bits)) & 63) as usize] as char);
    }
    uri
}

pub(super) fn parse(
    encoded: &str,
    now: u64,
    ports: &[u16],
) -> Result<PairingQrPayload, PairingQrError> {
    if !matches!(encoded.len(), 106 | 122) {
        return Err(PairingQrError::InvalidUri);
    }
    let mut bytes = Vec::with_capacity(91);
    let mut accumulator = 0_u32;
    let mut bits = 0;
    for byte in encoded.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(PairingQrError::InvalidUri),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((accumulator >> bits) as u8);
        }
    }
    if bits != 0 && (accumulator & ((1 << bits) - 1)) != 0 {
        return Err(PairingQrError::InvalidUri);
    }
    let offset = match (bytes.first(), bytes.len()) {
        (Some(4), 79) => 5,
        (Some(6), 91) => 17,
        _ => return Err(PairingQrError::InvalidUri),
    };
    let ip = if offset == 5 {
        IpAddr::V4(Ipv4Addr::new(bytes[1], bytes[2], bytes[3], bytes[4]))
    } else {
        let octets: [u8; 16] = bytes[1..17]
            .try_into()
            .map_err(|_| PairingQrError::InvalidUri)?;
        IpAddr::V6(Ipv6Addr::from(octets))
    };
    let port = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
    let endpoint = parse_manual_ip_endpoint(&SocketAddr::new(ip, port).to_string(), ports)
        .map_err(PairingQrError::InvalidEndpoint)?;
    let offset = offset + 2;
    let device_id = DeviceId::from_bytes(
        bytes[offset..offset + 16]
            .try_into()
            .map_err(|_| PairingQrError::InvalidDeviceId)?,
    );
    let fingerprint = PeerFingerprint::from_bytes(
        bytes[offset + 16..offset + 48]
            .try_into()
            .map_err(|_| PairingQrError::InvalidFingerprint)?,
    );
    let nonce = PairingNonce::from_bytes(
        bytes[offset + 48..offset + 64]
            .try_into()
            .map_err(|_| PairingQrError::InvalidNonce)?,
    );
    let expires_at_unix_seconds = u64::from_be_bytes(
        bytes[offset + 64..offset + 72]
            .try_into()
            .map_err(|_| PairingQrError::InvalidExpiry)?,
    );
    if expires_at_unix_seconds <= now {
        return Err(PairingQrError::Expired);
    }
    if expires_at_unix_seconds - now > PAIRING_QR_TTL_SECONDS {
        return Err(PairingQrError::ExpiryTooFar);
    }
    Ok(PairingQrPayload {
        endpoint,
        device_id,
        fingerprint,
        nonce,
        expires_at_unix_seconds,
    })
}
