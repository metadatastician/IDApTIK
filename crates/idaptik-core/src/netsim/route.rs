//! Ping and traceroute. Deterministic: fixed synthetic hops and latencies, no
//! wall-clock, no RNG (spec A.5).
use crate::netsim::access::can_reach;
use crate::netsim::graph::GroundedGraph;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// One traceroute hop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hop {
    pub ip: Ipv4Addr,
    pub name: String,
    pub latency_ms: u32,
}

/// Whether `dst` answers a ping from `src` (reachability).
pub fn ping(graph: &GroundedGraph, src: Ipv4Addr, dst: Ipv4Addr) -> bool {
    can_reach(graph, src, dst)
}

/// The hop path from `src` to `dst`, or empty if unreachable.
pub fn traceroute(
    graph: &GroundedGraph,
    src: Ipv4Addr,
    dst: Ipv4Addr,
    router_ip: Ipv4Addr,
) -> Vec<Hop> {
    if !can_reach(graph, src, dst) {
        return vec![];
    }
    let mut hops = Vec::new();
    if src != router_ip {
        hops.push(Hop {
            ip: router_ip,
            name: "WIFI-ROUTER".into(),
            latency_ms: 1,
        });
    }
    let registered = graph.nodes.iter().find(|n| n.ip == dst);
    // External-ness is segment membership, not node registration: an unregistered
    // host inside a known segment is still on-site, and a registered node that
    // sits in no segment is still out on the public internet.
    let external = crate::netsim::addressing::segment_of(graph, dst).is_none();
    if external {
        hops.push(Hop {
            ip: Ipv4Addr::new(10, 255, 255, 1),
            name: "isp-gateway".into(),
            latency_ms: 12,
        });
        hops.push(Hop {
            ip: Ipv4Addr::new(72, 14, 215, 85),
            name: "core-router-1".into(),
            latency_ms: 18,
        });
        hops.push(Hop {
            ip: Ipv4Addr::new(209, 85, 251, 9),
            name: "edge-router".into(),
            latency_ms: 24,
        });
    }
    let name = registered
        .map(|n| n.name.clone())
        .unwrap_or_else(|| dst.to_string());
    hops.push(Hop {
        ip: dst,
        name,
        latency_ms: if external { 32 } else { 2 },
    });
    hops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceKind, SecurityLevel};
    use crate::netsim::graph::{GroundedGraph, Node, Segment};
    use crate::network::{Range, Zone};
    use std::net::Ipv4Addr;

    fn graph() -> GroundedGraph {
        GroundedGraph {
            segments: vec![Segment {
                id: "lan".into(),
                range: Range::LocalLan,
                category: Zone::Lan,
                subnet: "192.168.1.".into(),
                can_access: vec!["lan".into(), "public".into()],
                location: None,
            }],
            nodes: vec![Node {
                id: "router".into(),
                name: "ROUTER".into(),
                ip: Ipv4Addr::new(192, 168, 1, 1),
                segment: "lan".into(),
                kind: DeviceKind::Router,
                security: SecurityLevel::Medium,
                actuation: None,
                deps: vec![],
            }],
            dns: vec![],
            vantages: vec![],
        }
    }

    #[test]
    fn traceroute_is_deterministic_and_gated() {
        let g = graph();
        let router = Ipv4Addr::new(192, 168, 1, 1);
        // Same inputs, identical output.
        let a = traceroute(&g, router, router, router);
        let b = traceroute(&g, router, router, router);
        assert_eq!(a, b);
        // router -> router (identity) is reachable and yields the single final hop.
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].latency_ms, 2);
    }

    #[test]
    fn traceroute_external_destination() {
        let g = graph();
        let router = Ipv4Addr::new(192, 168, 1, 1);
        let client = Ipv4Addr::new(192, 168, 1, 100);
        let external = Ipv4Addr::new(8, 8, 8, 8);
        // Trace from client to external destination; should include WIFI-ROUTER and synthetic hops.
        let hops = traceroute(&g, client, external, router);
        // Expected: WIFI-ROUTER (1) + isp-gateway (2) + core-router-1 (3) + edge-router (4) + final (5).
        assert_eq!(hops.len(), 5);
        // First hop: WIFI-ROUTER from client perspective.
        assert_eq!(hops[0].ip, router);
        assert_eq!(hops[0].name, "WIFI-ROUTER");
        assert_eq!(hops[0].latency_ms, 1);
        // Synthetic ISP gateway.
        assert_eq!(hops[1].ip, Ipv4Addr::new(10, 255, 255, 1));
        assert_eq!(hops[1].name, "isp-gateway");
        assert_eq!(hops[1].latency_ms, 12);
        // Synthetic core router.
        assert_eq!(hops[2].ip, Ipv4Addr::new(72, 14, 215, 85));
        assert_eq!(hops[2].name, "core-router-1");
        assert_eq!(hops[2].latency_ms, 18);
        // Synthetic edge router.
        assert_eq!(hops[3].ip, Ipv4Addr::new(209, 85, 251, 9));
        assert_eq!(hops[3].name, "edge-router");
        assert_eq!(hops[3].latency_ms, 24);
        // Final external destination.
        assert_eq!(hops[4].ip, external);
        assert_eq!(hops[4].name, "8.8.8.8");
        assert_eq!(hops[4].latency_ms, 32);
    }

    #[test]
    fn traceroute_from_non_router() {
        let g = graph();
        let router = Ipv4Addr::new(192, 168, 1, 1);
        let client = Ipv4Addr::new(192, 168, 1, 100);
        // Trace from client to router; first hop should be the WIFI-ROUTER.
        let hops = traceroute(&g, client, router, router);
        assert_eq!(hops.len(), 2);
        // First hop: WIFI-ROUTER hop.
        assert_eq!(hops[0].ip, router);
        assert_eq!(hops[0].name, "WIFI-ROUTER");
        assert_eq!(hops[0].latency_ms, 1);
        // Final hop: registered router (not external, so latency 2).
        assert_eq!(hops[1].ip, router);
        assert_eq!(hops[1].name, "ROUTER");
        assert_eq!(hops[1].latency_ms, 2);
    }

    #[test]
    fn an_unregistered_ip_inside_a_known_segment_is_not_external() {
        // Segment membership, not node registration, decides external-ness: a
        // host the graph does not list but whose address sits inside the LAN
        // subnet must not be routed out through the synthetic backbone.
        let g = graph();
        let router = Ipv4Addr::new(192, 168, 1, 1);
        let unregistered = Ipv4Addr::new(192, 168, 1, 77);
        let hops = traceroute(&g, router, unregistered, router);
        assert_eq!(hops.len(), 1, "no backbone hops inside the LAN");
        assert_eq!(hops[0].ip, unregistered);
        assert_eq!(hops[0].name, "192.168.1.77", "unnamed, so the dotted IP");
        assert_eq!(hops[0].latency_ms, 2, "on-site latency, not external");
    }

    #[test]
    fn a_registered_node_in_no_segment_is_external() {
        // The converse: registration does not make an address local. A node the
        // graph lists but whose IP falls in no segment subnet is on the public
        // internet, and the trace to it crosses the synthetic backbone — while
        // the final hop still carries the registered node's name.
        let mut g = graph();
        g.nodes.push(Node {
            id: "mirror".into(),
            name: "OFFSITE MIRROR".into(),
            ip: Ipv4Addr::new(198, 51, 100, 7),
            segment: "lan".into(),
            kind: DeviceKind::Server,
            security: SecurityLevel::Medium,
            actuation: None,
            deps: vec![],
        });
        let router = Ipv4Addr::new(192, 168, 1, 1);
        let dst = Ipv4Addr::new(198, 51, 100, 7);
        let hops = traceroute(&g, router, dst, router);
        assert_eq!(hops.len(), 4, "backbone hops plus the destination");
        assert_eq!(hops[0].name, "isp-gateway");
        assert_eq!(hops[3].ip, dst);
        assert_eq!(hops[3].name, "OFFSITE MIRROR", "the registration names it");
        assert_eq!(hops[3].latency_ms, 32, "external latency");
    }

    #[test]
    fn traceroute_unreachable() {
        let mut g = graph();
        // Add an isolated segment unreachable from the LAN.
        g.segments.push(Segment {
            id: "isolated".into(),
            range: Range::LocalLan,
            category: Zone::Lan,
            subnet: "10.0.0.".into(),
            can_access: vec![],
            location: None,
        });
        let router = Ipv4Addr::new(192, 168, 1, 1);
        let unreachable = Ipv4Addr::new(10, 0, 0, 1);
        // Trace to unreachable destination; should return empty.
        let hops = traceroute(&g, router, unreachable, router);
        assert_eq!(hops.len(), 0);
    }
}
