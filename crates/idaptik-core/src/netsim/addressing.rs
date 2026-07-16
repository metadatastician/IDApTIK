//! Mapping an IP address to the segment it belongs to (the port of IDApixiTIK's
//! `getZoneByIp`): a first-match dotted-prefix scan over the segments, skipping
//! the empty-subnet public sink so it never matches.
use crate::netsim::graph::{GroundedGraph, Segment};
use std::net::Ipv4Addr;

/// The segment an IP sits in, or `None` if it matches no segment (raw public IP).
pub fn segment_of(graph: &GroundedGraph, ip: Ipv4Addr) -> Option<&Segment> {
    let dotted = ip.to_string();
    // A segment's subnet is a dotted prefix such as "10.0.0."; the empty prefix
    // (the public sink) is skipped so it never matches.
    graph
        .segments
        .iter()
        .filter(|s| !s.subnet.is_empty())
        .find(|s| dotted.starts_with(&s.subnet))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::graph::{GroundedGraph, Segment};
    use crate::network::{Range, Zone};
    use std::net::Ipv4Addr;

    fn seg(id: &str, subnet: &str) -> Segment {
        Segment {
            id: id.into(),
            range: Range::WideArea,
            category: Zone::Service,
            subnet: subnet.into(),
            can_access: vec![],
            location: None,
        }
    }

    #[test]
    fn matches_by_prefix_and_leaves_public_unmatched() {
        let g = GroundedGraph {
            segments: vec![seg("dmz", "10.0.0."), seg("public", "")],
            nodes: vec![],
            dns: vec![],
            vantages: vec![],
        };
        assert_eq!(
            segment_of(&g, Ipv4Addr::new(10, 0, 0, 25)).unwrap().id,
            "dmz"
        );
        // The empty-subnet public sink never matches by prefix.
        assert!(segment_of(&g, Ipv4Addr::new(203, 0, 113, 9)).is_none());
    }
}
