//! The building's grounded network, derived from the definition — the same
//! move [`crate::scenario::floor_graph`] makes for the Ghost Lobby floor.
//!
//! Every [`crate::scenario::building::ZoneDef`] becomes one `netsim`
//! [`Segment`]; every circuit's feed and every portal's lock controller
//! becomes a [`Node`] on the segment of the zone it sits in, with the
//! controller drawing power from its portal's circuit. Nothing here reinvents
//! graph logic: reachability, DNS and [`crate::netsim::apply_actuation`] all
//! run over the ordinary [`GroundedGraph`].

use crate::device::{DeviceKind, SecurityLevel};
use crate::netsim::graph::{Actuation, Dependency, GroundedGraph, Node, Segment};
use crate::network::{Range, Zone};
use crate::scenario::building::{BuildingDefinition, RoomRef, ZoneDef};
use std::net::Ipv4Addr;

/// Host octet base of circuit feeds within their zone's subnet.
const FEED_HOST_BASE: usize = 2;

/// Host octet base of lock controllers within their zone's subnet.
const CONTROLLER_HOST_BASE: usize = 10;

/// The largest usable host octet in a dotted /24-style subnet prefix.
const MAX_HOST_OCTET: usize = 254;

/// An address built from a zone's dotted subnet prefix and a host octet, or
/// `None` if the octet overflows the subnet (or the prefix is malformed).
/// Deliberately non-panicking, exactly like the floor graph's addressing: a
/// definition that slips past validation degrades to a missing node, never a
/// panic in the sim.
fn host_ip(subnet: &str, host: usize) -> Option<Ipv4Addr> {
    if host > MAX_HOST_OCTET {
        return None;
    }
    format!("{subnet}{host}").parse().ok()
}

/// The zone serving a room, if any.
pub fn zone_of_room<'a>(def: &'a BuildingDefinition, room: &RoomRef) -> Option<&'a ZoneDef> {
    def.zones.iter().find(|z| z.rooms.contains(room))
}

/// The building's grounded network: one segment per zone, one feed node per
/// circuit, one controller node per locked portal.
pub fn building_network(def: &BuildingDefinition) -> GroundedGraph {
    let mut graph = GroundedGraph {
        segments: Vec::new(),
        nodes: Vec::new(),
        dns: Vec::new(),
        vantages: Vec::new(),
    };

    for zone in &def.zones {
        graph.segments.push(Segment {
            id: zone.id.as_str().to_owned(),
            range: Range::LocalLan,
            category: Zone::Internal,
            subnet: zone.subnet.clone(),
            can_access: Vec::new(),
            location: None,
        });
    }

    // Per-zone host allocation cursors, keyed by zone rank (definition order).
    let mut feed_rank: Vec<usize> = vec![0; def.zones.len()];
    let mut controller_rank: Vec<usize> = vec![0; def.zones.len()];
    let zone_rank = |id: &crate::scenario::ids::ZoneId| -> Option<usize> {
        def.zones.iter().position(|z| &z.id == id)
    };

    // Circuit feeds: medium-security power stations, no upstream dependency —
    // they are the building's roots, and cutting one cascades downward.
    for circuit in &def.circuits {
        let Some(rank) = zone_rank(&circuit.zone) else {
            continue;
        };
        let Some(zone) = def.zones.get(rank) else {
            continue;
        };
        let Some(ip) = host_ip(&zone.subnet, FEED_HOST_BASE + feed_rank[rank]) else {
            continue;
        };
        feed_rank[rank] += 1;
        graph.nodes.push(Node {
            id: circuit.source.clone(),
            name: format!("{} FEED", circuit.name),
            ip,
            segment: zone.id.as_str().to_owned(),
            kind: DeviceKind::PowerStation,
            security: SecurityLevel::Medium,
            actuation: Some(Actuation::CutPower),
            deps: Vec::new(),
        });
    }

    // Lock controllers: weakly secured, on the zone of the portal's near room
    // (falling back to the far room), drawing from the portal's circuit.
    for portal in &def.portals {
        let Some(lock) = &portal.lock else {
            continue;
        };
        let Some(zone) = zone_of_room(def, &portal.from).or_else(|| zone_of_room(def, &portal.to))
        else {
            continue;
        };
        let Some(rank) = zone_rank(&zone.id) else {
            continue;
        };
        let Some(ip) = host_ip(&zone.subnet, CONTROLLER_HOST_BASE + controller_rank[rank]) else {
            continue;
        };
        controller_rank[rank] += 1;
        let deps = portal
            .circuit
            .as_ref()
            .and_then(|c| def.circuit(c))
            .map(|c| {
                vec![Dependency {
                    on: c.source.clone(),
                }]
            })
            .unwrap_or_default();
        graph.nodes.push(Node {
            id: lock.controller.clone(),
            name: format!("{} LOCK", portal.label),
            ip,
            segment: zone.id.as_str().to_owned(),
            kind: DeviceKind::Lock,
            security: SecurityLevel::Weak,
            actuation: Some(Actuation::DisengageLock),
            deps,
        });
    }

    graph
}
