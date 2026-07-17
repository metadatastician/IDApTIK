//! Engine-agnostic gameplay truth for IDApTIK.
//!
//! This crate holds the authoritative simulation the two roles share: the
//! network the hacker navigates, the devices they compromise, and the trace
//! clock they race. It has **no rendering dependency** — Bevy and Fyrox are
//! frontends over this (ADR-0003), and the Elixir session layer coordinates it
//! (ADR-0002). Keep game logic here, not in a frontend.
//!
//! The [`scenario`] module ports the "Envelope 001 – Ghost Lobby" prototype as a
//! deterministic, event-sourced, definition-as-data scenario (ADR-0004): content
//! is a serde [`ScenarioDefinition`], the simulation consumes typed [`Command`]s
//! and emits typed [`Event`]s, and every run is reproducible byte-for-byte from
//! `(definition, config, seed, command stream)`.
#![forbid(unsafe_code)]

pub mod companion;
pub mod device;
pub mod netsim;
pub mod network;
pub mod scenario;
pub mod trace;

pub use companion::{
    CompanionDefinition, MOLETAIRE_JSON, MoleCommand, MoleEvent, MoleParams, MoletaireSim,
    MoletaireSnapshot, moletaire,
};
pub use device::{Device, DeviceId, DeviceKind, SecurityLevel};
pub use network::{Network, Range, Zone};
pub use scenario::{
    ACTORS_JSON, ActorArchetype, ActorRegistry, Buttons, Command, ComposedActor, Debrief,
    DifficultyId, Event, GHOST_LOBBY_JSON, GhostLobbySim, LogLine, Modifier, Mulberry32, RunConfig,
    RuntimeSnapshot, ScenarioDefinition, ScenarioExport, TickInput, ValidationError,
    default_registry, ghost_lobby, load_actor_pack,
};
pub use trace::{Alert, Trace};

use std::net::Ipv4Addr;

/// A small demonstration network, used by the frontends to have something real
/// to show and by the test suite. Roughly the segmented topology described in
/// the IDApTIK notes: a DMZ reachable from outside, a protected internal
/// segment behind it, and a weak IoT segment that bridges to the physical layer.
pub fn demo_network() -> Network {
    use DeviceKind::*;
    use SecurityLevel::*;
    use Zone::*;

    let mut net = Network::new();

    // (id, name, ip, kind, security, zone)
    let devices = [
        (
            0,
            "edge-router",
            Ipv4Addr::new(10, 0, 0, 1),
            Router,
            Medium,
            Dmz,
        ),
        (1, "web", Ipv4Addr::new(10, 0, 0, 10), Server, Weak, Dmz),
        (
            2,
            "db",
            Ipv4Addr::new(10, 0, 1, 10),
            Server,
            Strong,
            Internal,
        ),
        (
            3,
            "ops-terminal",
            Ipv4Addr::new(10, 0, 1, 20),
            Terminal,
            Medium,
            Internal,
        ),
        (
            4,
            "lobby-cam",
            Ipv4Addr::new(192, 168, 100, 5),
            IotCamera,
            Open,
            Iot,
        ),
        (5, "ups", Ipv4Addr::new(10, 10, 0, 2), Ups, Weak, Scada),
    ];
    for (id, name, ip, kind, security, zone) in devices {
        net.add_device(Device::new(DeviceId(id), name, ip, kind, security, zone));
    }

    // Star-ish topology hung off the edge router.
    net.link(DeviceId(0), DeviceId(1)); // router -> web (DMZ)
    net.link(DeviceId(1), DeviceId(2)); // web -> db (the pivot into internal)
    net.link(DeviceId(2), DeviceId(3)); // db -> ops terminal
    net.link(DeviceId(0), DeviceId(4)); // router -> IoT camera
    net.link(DeviceId(4), DeviceId(5)); // camera -> UPS (IoT bridges to SCADA)

    net
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_network_is_connected_and_pivotable() {
        let net = demo_network();
        assert_eq!(net.len(), 6);
        // The prized internal db is not directly external — it takes a pivot.
        let path = net
            .hop_path(DeviceId(0), DeviceId(2))
            .expect("db is reachable via a pivot chain");
        assert_eq!(path, vec![DeviceId(0), DeviceId(1), DeviceId(2)]);
        // From the edge router the hacker can eventually see the whole estate.
        assert_eq!(net.reachable_from(DeviceId(0)).len(), 6);
    }
}
