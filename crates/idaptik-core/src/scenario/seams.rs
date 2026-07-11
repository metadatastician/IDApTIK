//! Reflective seams to the rest of the core crate — documented, and **never**
//! called by the tick pipeline. They let the Ghost Lobby floor project itself
//! into the hacker-facing [`crate::Network`] and the [`crate::Trace`] clock, so
//! it can later slot into the UMS building runtime without the sim depending on
//! those subsystems.

use crate::device::{Device, DeviceId, DeviceKind, SecurityLevel};
use crate::network::{Network, Zone};
use crate::scenario::definition::ScenarioDefinition;
use crate::trace::Trace;

/// Project a scenario definition into a hacker-facing network view: each camera
/// becomes an IoT camera, each door controller a terminal, hung off a root
/// laptop/router. Purely derived; the sim never consults it.
pub fn network_view(def: &ScenarioDefinition) -> Network {
    let mut net = Network::new();
    let root = DeviceId(0);
    net.add_device(Device::new(
        root,
        "uplink-laptop",
        std::net::Ipv4Addr::new(10, 20, 0, 1),
        DeviceKind::Router,
        SecurityLevel::Medium,
        Zone::Dmz,
    ));

    let mut next = 1u32;
    for cam in &def.cameras {
        let id = DeviceId(next);
        next += 1;
        net.add_device(Device::new(
            id,
            cam.id.as_str(),
            std::net::Ipv4Addr::new(192, 168, 100, next as u8),
            DeviceKind::IotCamera,
            SecurityLevel::Open,
            Zone::Iot,
        ));
        net.link(root, id);
    }
    for door in &def.doors {
        let id = DeviceId(next);
        next += 1;
        net.add_device(Device::new(
            id,
            door.id.as_str(),
            std::net::Ipv4Addr::new(10, 20, 1, next as u8),
            DeviceKind::Terminal,
            SecurityLevel::Weak,
            Zone::Internal,
        ));
        net.link(root, id);
    }
    net
}

/// Project the current alert level onto a [`Trace`] clock: `threshold` is the
/// alert ceiling and the alert becomes the trace progress. A convenience for
/// frontends that want to reuse the trace-bar widget; not part of determinism.
pub fn trace_from_alert(alert: f64, threshold: u32) -> Trace {
    let mut trace = Trace::new(threshold);
    let clamped = alert.clamp(0.0, f64::from(threshold));
    trace.advance(clamped.round() as u32, 1);
    trace
}
