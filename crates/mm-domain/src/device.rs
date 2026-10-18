use crate::{CapabilitySet, DeviceId, DomainError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceRole(u8);

impl DeviceRole {
    const SEND: u8 = 0b01;
    const RECEIVE: u8 = 0b10;

    pub const fn sender() -> Self {
        Self(Self::SEND)
    }

    pub const fn receiver() -> Self {
        Self(Self::RECEIVE)
    }

    pub const fn sender_and_receiver() -> Self {
        Self(Self::SEND | Self::RECEIVE)
    }

    pub const fn can_send(self) -> bool {
        self.0 & Self::SEND != 0
    }

    pub const fn can_receive(self) -> bool {
        self.0 & Self::RECEIVE != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    pub display_name: String,
    pub role: DeviceRole,
    pub capabilities: CapabilitySet,
}

impl Device {
    pub fn new(
        id: DeviceId,
        display_name: String,
        role: DeviceRole,
        capabilities: CapabilitySet,
    ) -> Result<Self, DomainError> {
        let name_valid = !display_name.is_empty()
            && display_name.chars().count() <= 80
            && !display_name.chars().any(char::is_control);
        if !name_valid {
            return Err(DomainError::InvalidDisplayName);
        }
        Ok(Self {
            id,
            display_name,
            role,
            capabilities,
        })
    }
}
