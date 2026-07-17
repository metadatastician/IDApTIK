//! Plain-text rendering for the net mode: vantage, trace, reachable nodes, logs.
use idaptik_core::netsim::{AgentSession, GroundedGraph};

/// Draw the current state to stdout.
pub fn screen(graph: &GroundedGraph, session: &AgentSession) {
    println!("\n== grounded network ==");
    println!("vantage host: {}", session.vantage_ip());
    println!("trace: {:.0}%", session.trace_fraction() * 100.0);
    println!("reachable from here:");
    for ip in session.reachable(graph) {
        if let Some(node) = graph.nodes.iter().find(|n| n.ip == ip) {
            let act = node
                .actuation
                .map(|a| format!(" [{a:?}]"))
                .unwrap_or_default();
            println!("  {:<16} {:<18} {:?}{}", node.ip, node.name, node.kind, act);
        }
    }
    if !session.logs().is_empty() {
        println!("logs left: {:?}", session.logs());
    }
}
