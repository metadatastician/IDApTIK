//! Reachability from a given origin, evaluated by the access matrix from that
//! origin's segment. Pivoting changes the origin, which is how bouncing crosses
//! more than one boundary.
use crate::netsim::access::segment_can_reach;
use crate::netsim::addressing::segment_of;
use crate::netsim::graph::GroundedGraph;
use std::net::Ipv4Addr;

/// Every node reachable from `from` (excluding `from`), in declaration order.
///
/// The origin's segment is resolved once here rather than once per node: the
/// answer is the same for every destination, and resolving it is the expensive
/// half of the decision.
fn reachable_iter(graph: &GroundedGraph, from: Ipv4Addr) -> impl Iterator<Item = Ipv4Addr> + '_ {
    let src = segment_of(graph, from);
    graph
        .nodes
        .iter()
        .map(|n| n.ip)
        .filter(move |&ip| ip != from && segment_can_reach(graph, src, ip))
}

/// Every node reachable from `from` (excluding `from`), in declaration order.
pub fn reachable_from(graph: &GroundedGraph, from: Ipv4Addr) -> Vec<Ipv4Addr> {
    reachable_iter(graph, from).collect()
}

/// How many nodes are reachable from `from`, for a caller that wants the tally
/// and not the list: it counts without collecting them.
pub fn reachable_count_from(graph: &GroundedGraph, from: Ipv4Addr) -> usize {
    reachable_iter(graph, from).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceKind, SecurityLevel};
    use crate::netsim::graph::{GroundedGraph, Node, Segment};
    use crate::network::{Range, Zone};
    use std::net::Ipv4Addr;

    fn node(id: &str, ip: [u8; 4], seg: &str) -> Node {
        Node {
            id: id.into(),
            name: id.into(),
            ip: Ipv4Addr::from(ip),
            segment: seg.into(),
            kind: DeviceKind::Server,
            security: SecurityLevel::Weak,
            actuation: None,
            deps: vec![],
        }
    }
    fn seg(id: &str, subnet: &str, access: &[&str]) -> Segment {
        Segment {
            id: id.into(),
            range: Range::LocalLan,
            category: Zone::Internal,
            subnet: subnet.into(),
            can_access: access.iter().map(|s| s.to_string()).collect(),
            location: None,
        }
    }

    #[test]
    fn reachability_follows_the_current_segment() {
        let g = GroundedGraph {
            segments: vec![
                seg("dmz", "10.0.0.", &["internal"]),
                seg("internal", "10.0.1.", &[]),
            ],
            nodes: vec![
                node("web", [10, 0, 0, 25], "dmz"),
                node("db", [10, 0, 1, 50], "internal"),
            ],
            dns: vec![],
            vantages: vec![],
        };
        // From the DMZ web host, the internal db is reachable (dmz -> internal edge).
        let from_dmz = reachable_from(&g, Ipv4Addr::new(10, 0, 0, 25));
        assert!(from_dmz.contains(&Ipv4Addr::new(10, 0, 1, 50)));
        // From the internal db, the dmz web host is NOT (internal has no edges).
        let from_internal = reachable_from(&g, Ipv4Addr::new(10, 0, 1, 50));
        assert!(!from_internal.contains(&Ipv4Addr::new(10, 0, 0, 25)));
    }
}
