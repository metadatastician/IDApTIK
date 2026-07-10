//! Devices the hacker discovers, compromises, and pivots through.

use crate::network::Zone;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Stable identifier for a [`Device`] within a [`crate::Network`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceId(pub u32);

/// The kind of device on the network. Mirrors the taxonomy from IDApTIK's
/// `DeviceTypes` (Laptop, Router, Server, IoT camera, Terminal, PowerStation, UPS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceKind {
    Laptop,
    Router,
    Server,
    IotCamera,
    Terminal,
    PowerStation,
    Ups,
}

/// How hard a device is to compromise. Ordered from easiest to hardest, so
/// comparisons like `security >= SecurityLevel::Medium` are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    Open,
    Weak,
    Medium,
    Strong,
}

/// A single device on the network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub ip: Ipv4Addr,
    pub kind: DeviceKind,
    pub security: SecurityLevel,
    pub zone: Zone,
    /// Whether the hacker currently controls this device (i.e. it can be used as
    /// a pivot / foothold to attack from).
    pub compromised: bool,
}

impl Device {
    /// Create an un-compromised device.
    pub fn new(
        id: DeviceId,
        name: impl Into<String>,
        ip: Ipv4Addr,
        kind: DeviceKind,
        security: SecurityLevel,
        zone: Zone,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            ip,
            kind,
            security,
            zone,
            compromised: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_levels_are_ordered() {
        assert!(SecurityLevel::Open < SecurityLevel::Strong);
        assert!(SecurityLevel::Medium >= SecurityLevel::Weak);
    }

    #[test]
    fn new_device_starts_uncompromised() {
        let d = Device::new(
            DeviceId(1),
            "web",
            Ipv4Addr::new(10, 0, 0, 10),
            DeviceKind::Server,
            SecurityLevel::Weak,
            Zone::Dmz,
        );
        assert!(!d.compromised);
    }
}
