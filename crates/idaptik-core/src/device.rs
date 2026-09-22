//! Devices the hacker discovers, compromises, and pivots through.

use crate::network::Zone;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Stable identifier for a [`Device`] within a [`crate::Network`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceId(pub u32);

/// The kind of device on the network. The original taxonomy (Laptop, Router,
/// Server, IoT camera, Terminal, PowerStation, UPS) plus a Firewall, the
/// physical-function devices the hacker actuates, and the UMS editor kinds
/// (below the marker comment) so `idaptik-edit/1` content maps 1:1 into the
/// game with no lossy remapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceKind {
    Laptop,
    Router,
    Server,
    IotCamera,
    Terminal,
    PowerStation,
    Ups,
    Firewall,
    SmartDoor,
    Camera,
    Lock,
    Elevator,
    Light,
    Sensor,
    Substation,
    // UMS parity (idaptik-edit/1): kinds the level editor authors that the game
    // taxonomy lacked. Appended so existing wire tags and orderings are
    // untouched. Passive plant (PatchPanel, FibreHub, PowerSupply) carries no
    // actuation; infrastructure (Switch, AccessPoint) plays a Router-like
    // topology role; Desktop is an end-user host like Laptop/Terminal.
    PatchPanel,
    FibreHub,
    PhoneSystem,
    AccessPoint,
    Switch,
    Desktop,
    PowerSupply,
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

    /// The UMS↔game seam (idaptik-edit/1) maps device kinds by these exact
    /// wire tags; a rename here silently breaks every authored DLC manifest.
    #[test]
    fn ums_parity_kinds_have_stable_wire_tags() {
        let tags = [
            (DeviceKind::PatchPanel, "\"PatchPanel\""),
            (DeviceKind::FibreHub, "\"FibreHub\""),
            (DeviceKind::PhoneSystem, "\"PhoneSystem\""),
            (DeviceKind::AccessPoint, "\"AccessPoint\""),
            (DeviceKind::Switch, "\"Switch\""),
            (DeviceKind::Desktop, "\"Desktop\""),
            (DeviceKind::PowerSupply, "\"PowerSupply\""),
        ];
        for (kind, tag) in tags {
            let json = serde_json::to_string(&kind).expect("kinds serialize");
            assert_eq!(json, tag);
            let back: DeviceKind = serde_json::from_str(&json).expect("kinds deserialize");
            assert_eq!(back, kind);
        }
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
