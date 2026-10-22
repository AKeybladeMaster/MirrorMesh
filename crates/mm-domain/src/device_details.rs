//! Bounded, sender-reported device facts. These are identity-bound hints, not attestation.

use crate::DomainError;

pub const PAIRING_PROOF_DOMAIN_V1: &[u8] = b"MirrorMesh.PairingProof.v1\0";
pub const PAIRING_PROOF_DOMAIN_V2: &[u8] = b"MirrorMesh.PairingProof.v2\0";
pub const MAX_DEVICE_DETAIL_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportedDeviceDetails {
    platform: String,
    manufacturer: String,
    model: String,
    os_version: String,
}

impl ReportedDeviceDetails {
    pub fn new(
        platform: String,
        manufacturer: String,
        model: String,
        os_version: String,
    ) -> Result<Self, DomainError> {
        if platform.trim().is_empty()
            || [&platform, &manufacturer, &model, &os_version]
                .into_iter()
                .any(|value| !safe_detail(value))
        {
            return Err(DomainError::InvalidDisplayName);
        }
        Ok(Self {
            platform,
            manufacturer,
            model,
            os_version,
        })
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }
    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }
    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn os_version(&self) -> &str {
        &self.os_version
    }

    /// Fixed field order; byte lengths prevent concatenation ambiguity.
    pub fn append_canonical(&self, output: &mut Vec<u8>) {
        for value in [
            self.platform(),
            self.manufacturer(),
            self.model(),
            self.os_version(),
        ] {
            output.extend_from_slice(&(value.len() as u32).to_be_bytes());
            output.extend_from_slice(value.as_bytes());
        }
    }
}

fn safe_detail(value: &str) -> bool {
    value.chars().count() <= MAX_DEVICE_DETAIL_CHARS && !value.chars().any(|ch| {
        ch.is_control() || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    })
}

/// Shared signing boundary for platform identities; accepts only complete v1/v2 proofs.
/// Private keys must never sign arbitrary messages under either domain separator.
pub fn is_canonical_pairing_message(message: &[u8]) -> bool {
    let (body, details) = if let Some(body) = message.strip_prefix(PAIRING_PROOF_DOMAIN_V1) {
        (body, false)
    } else if let Some(body) = message.strip_prefix(PAIRING_PROOF_DOMAIN_V2) {
        (body, true)
    } else {
        return false;
    };
    let mut offset = 16 * 3;
    let Some(name) = take_text(body, &mut offset) else {
        return false;
    };
    if name.is_empty()
        || name.chars().count() > crate::MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return false;
    }
    let Some(&grants) = body.get(offset) else {
        return false;
    };
    if grants == 0 || grants & !0b111 != 0 {
        return false;
    }
    offset += 1;
    if details {
        for field in 0..4 {
            let Some(value) = take_text(body, &mut offset) else {
                return false;
            };
            if !safe_detail(value) || (field == 0 && value.trim().is_empty()) {
                return false;
            }
        }
    }
    offset == body.len()
}

fn take_text<'a>(body: &'a [u8], offset: &mut usize) -> Option<&'a str> {
    let length_end = offset.checked_add(4)?;
    let length = u32::from_be_bytes(body.get(*offset..length_end)?.try_into().ok()?) as usize;
    if length > MAX_DEVICE_DETAIL_CHARS * 4 {
        return None;
    }
    let end = length_end.checked_add(length)?;
    let value = std::str::from_utf8(body.get(length_end..end)?).ok()?;
    *offset = end;
    Some(value)
}
