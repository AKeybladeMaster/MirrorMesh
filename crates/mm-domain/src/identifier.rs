use std::fmt;

use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId([u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId([u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageId([u8; 16]);

impl DeviceId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        parse_uuid(value).map(Self)
    }
}

impl SessionId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        let bytes = parse_uuid(value)?;
        if !is_uuid_v4(&bytes) {
            return Err(DomainError::InvalidIdentifier);
        }
        Ok(Self(bytes))
    }
}

impl MessageId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        parse_uuid(value).map(Self)
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_uuid(&self.0, formatter)
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_uuid(&self.0, formatter)
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_uuid(&self.0, formatter)
    }
}

fn parse_uuid(value: &str) -> Result<[u8; 16], DomainError> {
    if value.len() != 36
        || value.as_bytes()[8] != b'-'
        || value.as_bytes()[13] != b'-'
        || value.as_bytes()[18] != b'-'
        || value.as_bytes()[23] != b'-'
    {
        return Err(DomainError::InvalidIdentifier);
    }

    let mut output = [0_u8; 16];
    let mut high_nibble = None;
    let mut output_index = 0;
    for (index, byte) in value.bytes().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(DomainError::InvalidIdentifier),
        };
        if let Some(high) = high_nibble.take() {
            output[output_index] = (high << 4) | nibble;
            output_index += 1;
        } else {
            high_nibble = Some(nibble);
        }
    }
    if output_index != output.len() || high_nibble.is_some() {
        return Err(DomainError::InvalidIdentifier);
    }
    Ok(output)
}

fn is_uuid_v4(bytes: &[u8; 16]) -> bool {
    bytes[6] >> 4 == 4 && bytes[8] & 0xc0 == 0x80
}

fn format_uuid(bytes: &[u8; 16], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            formatter.write_str("-")?;
        }
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DeviceId, SessionId};

    #[test]
    fn parses_and_formats_canonical_uuid() {
        let parsed = DeviceId::parse("8b3d8788-1476-4aae-a287-334a428b5b6f").unwrap();
        assert_eq!(parsed.to_string(), "8b3d8788-1476-4aae-a287-334a428b5b6f");
        assert_eq!(parsed.as_bytes()[0], 0x8b);
    }

    #[test]
    fn rejects_invalid_uuid_shapes_and_characters() {
        assert!(DeviceId::parse("not-a-uuid").is_err());
        assert!(SessionId::parse("8b3d878814764aaea287334a428b5b6f").is_err());
        assert!(SessionId::parse("8b3d8788-1476-4aae-a287-334a428b5b6g").is_err());
    }

    #[test]
    fn session_ids_must_use_uuid_v4_variant_bits() {
        assert!(SessionId::parse("8b3d8788-1476-4aae-a287-334a428b5b6f").is_ok());
        assert!(SessionId::parse("8b3d8788-1476-7aae-a287-334a428b5b6f").is_err());
        assert!(SessionId::parse("8b3d8788-1476-4aae-2287-334a428b5b6f").is_err());
    }
}
