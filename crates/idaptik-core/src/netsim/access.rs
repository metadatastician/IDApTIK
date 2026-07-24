//! The zone-segment access decision (the port of `canIpAccessIp`): single-hop
//! adjacency plus the ISP/service shortcut. No transitive routing; crossing more
//! than one boundary is done by pivoting.
use crate::netsim::addressing::segment_of;
use crate::netsim::graph::{GroundedGraph, Segment};
use crate::network::Zone;
use std::net::Ipv4Addr;

const TIER3: [&str; 2] = ["isp-tier3-business", "isp-tier3-rural"];

fn can_zone_access_zone(src: &Segment, dst: &Segment) -> bool {
    src.id == dst.id || src.can_access.iter().any(|z| z == &dst.id)
}

fn reaches_any(src: &Segment, ids: &[&str]) -> bool {
    src.can_access.iter().any(|z| ids.contains(&z.as_str()))
}

fn can_route_via_isp(src: &Segment, dst: &Segment) -> bool {
    match dst.category {
        Zone::Service => {
            reaches_any(src, &["isp-tier1-backbone", "isp-tier2-regional"])
                || reaches_any(src, &TIER3)
                || src.can_access.iter().any(|z| z == "public")
        }
        Zone::Isp => {
            // The direct src.can_access edge is already covered by the caller's
            // can_zone_access_zone check, so only the shortcuts live here.
            reaches_any(src, &TIER3) || src.can_access.iter().any(|z| z == "public")
        }
        _ => false,
    }
}

/// Whether a source already resolved to `src` may reach `dst`. `src` is `None`
/// when the origin sits on no segment (a raw public IP), which reaches nothing.
///
/// Split out from [`can_reach`] so that a caller judging many destinations from
/// one origin resolves the source segment once rather than once per destination.
/// It does not carry the identity shortcut, since there is no source address here
/// to compare against; [`can_reach`] applies that before calling in.
pub(crate) fn segment_can_reach(
    graph: &GroundedGraph,
    src: Option<&Segment>,
    dst: Ipv4Addr,
) -> bool {
    match (src, segment_of(graph, dst)) {
        (Some(s), Some(d)) => can_zone_access_zone(s, d) || can_route_via_isp(s, d),
        (Some(s), None) => {
            // dest is a raw public IP: reachable iff the source has a way out.
            s.can_access.iter().any(|z| z == "public") || reaches_any(s, &TIER3)
        }
        _ => false,
    }
}

/// Whether `src` may reach `dst` from its current segment.
pub fn can_reach(graph: &GroundedGraph, src: Ipv4Addr, dst: Ipv4Addr) -> bool {
    if src == dst {
        return true;
    }
    segment_can_reach(graph, segment_of(graph, src), dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::graph::{GroundedGraph, Segment};
    use crate::network::{Range, Zone};
    use std::net::Ipv4Addr;

    fn seg(id: &str, cat: Zone, subnet: &str, access: &[&str]) -> Segment {
        Segment {
            id: id.into(),
            range: Range::LocalLan,
            category: cat,
            subnet: subnet.into(),
            can_access: access.iter().map(|s| s.to_string()).collect(),
            location: None,
        }
    }

    fn graph() -> GroundedGraph {
        GroundedGraph {
            segments: vec![
                seg("security", Zone::Internal, "10.0.3.", &["scada"]),
                seg("scada", Zone::Scada, "10.10.1.", &[]),
                seg("dmz", Zone::Dmz, "10.0.0.", &["internal"]),
                seg("internal", Zone::Internal, "10.0.1.", &[]),
            ],
            nodes: vec![],
            dns: vec![],
            vantages: vec![],
        }
    }

    #[test]
    fn single_hop_adjacency_permits_and_denies() {
        let g = graph();
        // security -> scada is a listed edge.
        assert!(can_reach(
            &g,
            Ipv4Addr::new(10, 0, 3, 10),
            Ipv4Addr::new(10, 10, 1, 1)
        ));
        // scada is air-gapped outbound; dmz cannot reach it (no edge).
        assert!(!can_reach(
            &g,
            Ipv4Addr::new(10, 10, 1, 1),
            Ipv4Addr::new(10, 0, 0, 25)
        ));
        assert!(!can_reach(
            &g,
            Ipv4Addr::new(10, 0, 0, 25),
            Ipv4Addr::new(10, 10, 1, 1)
        ));
    }

    #[test]
    fn identity_is_always_reachable() {
        let g = graph();
        let ip = Ipv4Addr::new(10, 0, 1, 50);
        assert!(can_reach(&g, ip, ip));
    }
}
