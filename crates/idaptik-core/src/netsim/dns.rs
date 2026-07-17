//! Name resolution over the graph's authoritative records. An input that already
//! parses as an IPv4 is returned unchanged (the port's short-circuit).
use crate::netsim::graph::GroundedGraph;
use std::net::Ipv4Addr;

/// Resolve a hostname (or literal IP) to an address, or `None` if unknown.
pub fn resolve(graph: &GroundedGraph, host: &str) -> Option<Ipv4Addr> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Some(ip);
    }
    graph.dns.iter().find(|r| r.host == host).map(|r| r.ip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::graph::{DnsRecord, GroundedGraph};
    use std::net::Ipv4Addr;

    fn graph() -> GroundedGraph {
        GroundedGraph {
            segments: vec![],
            nodes: vec![],
            dns: vec![DnsRecord {
                host: "secret-server.local".into(),
                ip: Ipv4Addr::new(10, 0, 1, 99),
            }],
            vantages: vec![],
        }
    }

    #[test]
    fn resolves_known_host_passes_through_ip_and_fails_unknown() {
        let g = graph();
        assert_eq!(
            resolve(&g, "secret-server.local"),
            Some(Ipv4Addr::new(10, 0, 1, 99))
        );
        assert_eq!(resolve(&g, "10.0.1.99"), Some(Ipv4Addr::new(10, 0, 1, 99)));
        assert_eq!(resolve(&g, "nope.local"), None);
    }
}
