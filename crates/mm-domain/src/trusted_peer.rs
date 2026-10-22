use crate::{DeviceId, DomainError};

pub const MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS: usize = 80;
pub const MAX_TRUSTED_PEER_PUBLIC_KEY_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerPublicKey(Vec<u8>);

impl PeerPublicKey {
    pub fn new(bytes: Vec<u8>) -> Result<Self, DomainError> {
        if bytes.is_empty() || bytes.len() > MAX_TRUSTED_PEER_PUBLIC_KEY_BYTES {
            return Err(DomainError::InvalidTrustedPeer);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerFingerprint([u8; 32]);

impl PeerFingerprint {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(64);
        for byte in self.0 {
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
        encoded
    }

    pub fn from_hex(encoded: &str) -> Result<Self, DomainError> {
        if encoded.len() != 64 {
            return Err(DomainError::InvalidTrustedPeer);
        }

        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            let high = decode_lower_hex(encoded.as_bytes()[offset])
                .ok_or(DomainError::InvalidTrustedPeer)?;
            let low = decode_lower_hex(encoded.as_bytes()[offset + 1])
                .ok_or(DomainError::InvalidTrustedPeer)?;
            *byte = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

fn decode_lower_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerGrant {
    View,
    Send,
    Annotate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedPeerGrants {
    view: bool,
    send: bool,
    annotate: bool,
}

impl TrustedPeerGrants {
    pub fn new(view: bool, send: bool, annotate: bool) -> Result<Self, DomainError> {
        if !view && !send && !annotate {
            return Err(DomainError::InvalidTrustedPeer);
        }
        Ok(Self {
            view,
            send,
            annotate,
        })
    }

    pub const fn allows(self, grant: PeerGrant) -> bool {
        match grant {
            PeerGrant::View => self.view,
            PeerGrant::Send => self.send,
            PeerGrant::Annotate => self.annotate,
        }
    }

    pub const fn is_subset_of(self, requested: Self) -> bool {
        (!self.view || requested.view)
            && (!self.send || requested.send)
            && (!self.annotate || requested.annotate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPeer {
    id: DeviceId,
    display_name: String,
    public_key: PeerPublicKey,
    fingerprint: PeerFingerprint,
    grants: TrustedPeerGrants,
    /// UTC Unix milliseconds; persisted values are not monotonic-clock readings.
    paired_at_ms: u64,
    /// UTC Unix milliseconds of the most recent authenticated contact.
    last_seen_at_ms: Option<u64>,
    /// UTC Unix milliseconds when user trust was revoked.
    revoked_at_ms: Option<u64>,
}

impl TrustedPeer {
    pub fn new(
        id: DeviceId,
        display_name: String,
        public_key: PeerPublicKey,
        fingerprint: PeerFingerprint,
        grants: TrustedPeerGrants,
        paired_at_ms: u64,
    ) -> Result<Self, DomainError> {
        let character_count = display_name.chars().count();
        if character_count == 0
            || character_count > MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS
            || display_name.chars().any(char::is_control)
        {
            return Err(DomainError::InvalidTrustedPeer);
        }
        Ok(Self {
            id,
            display_name,
            public_key,
            fingerprint,
            grants,
            paired_at_ms,
            last_seen_at_ms: None,
            revoked_at_ms: None,
        })
    }

    pub const fn id(&self) -> DeviceId {
        self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn public_key(&self) -> &PeerPublicKey {
        &self.public_key
    }

    pub const fn fingerprint(&self) -> PeerFingerprint {
        self.fingerprint
    }

    pub const fn grants(&self) -> TrustedPeerGrants {
        self.grants
    }

    pub const fn paired_at_ms(&self) -> u64 {
        self.paired_at_ms
    }

    pub const fn last_seen_at_ms(&self) -> Option<u64> {
        self.last_seen_at_ms
    }

    pub const fn revoked_at_ms(&self) -> Option<u64> {
        self.revoked_at_ms
    }

    pub const fn is_active(&self) -> bool {
        self.revoked_at_ms.is_none()
    }

    pub const fn allows(&self, grant: PeerGrant) -> bool {
        self.is_active() && self.grants.allows(grant)
    }

    pub fn record_seen(&mut self, now_unix_ms: u64) -> Result<(), DomainError> {
        if !self.is_active() {
            return Err(DomainError::TrustedPeerRevoked);
        }
        if now_unix_ms < self.paired_at_ms
            || self.last_seen_at_ms.is_some_and(|last| now_unix_ms < last)
        {
            return Err(DomainError::InvalidTrustedPeer);
        }
        self.last_seen_at_ms = Some(now_unix_ms);
        Ok(())
    }

    pub fn revoke(&mut self, revoked_at_unix_ms: u64) -> Result<bool, DomainError> {
        if self.revoked_at_ms.is_some() {
            return Ok(false);
        }
        if revoked_at_unix_ms < self.paired_at_ms
            || self
                .last_seen_at_ms
                .is_some_and(|last| revoked_at_unix_ms < last)
        {
            return Err(DomainError::InvalidTrustedPeer);
        }
        self.revoked_at_ms = Some(revoked_at_unix_ms);
        Ok(true)
    }
}
