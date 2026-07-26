//! Interactive loop for the net mode. A line-driven command prompt over the
//! grounded slice graph.
use crate::net::render;
use idaptik_core::DeviceKind;
use idaptik_core::netsim::{
    AgentSession, VantageDef, VantageKind, grounded_slice, ping, resolve, traceroute,
};
use std::io::{self, Write};

/// Run the net explorer to completion (until `quit`).
pub fn run(kind: VantageKind) -> io::Result<()> {
    let graph = grounded_slice();
    let vantage: VantageDef = graph
        .vantages
        .iter()
        .find(|v| v.kind == kind)
        .cloned()
        .unwrap_or_else(|| graph.vantages[0].clone());
    let mut session = AgentSession::new(vantage, 1000);

    let stdin = io::stdin();
    loop {
        render::screen(&graph, &session);
        print!("net> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("");
        match cmd {
            "quit" => break,
            "exit" => {
                if !session.exit() {
                    println!("already at the vantage");
                }
            }
            "ssh" => match session.ssh(&graph, arg) {
                Ok(ip) => println!("pivoted to {ip}"),
                Err(e) => println!("ssh failed: {e:?}"),
            },
            "hack" => match session.hack(&graph, arg) {
                Ok(effects) => println!("effects: {effects:?}"),
                Err(e) => println!("hack failed: {e:?}"),
            },
            "scrub" => {
                session.scrub(arg);
                println!("scrubbed {arg}");
            }
            "resolve" => match resolve(&graph, arg) {
                Some(ip) => println!("resolved {arg} -> {ip}"),
                None => println!("could not resolve {arg}"),
            },
            "ping" => match resolve(&graph, arg) {
                Some(ip) => {
                    if ping(&graph, session.vantage_ip(), ip) {
                        println!("{ip}: reachable");
                    } else {
                        println!("{ip}: no route");
                    }
                }
                None => println!("could not resolve {arg}"),
            },
            "traceroute" => match resolve(&graph, arg) {
                Some(ip) => {
                    let router_ip = graph
                        .nodes
                        .iter()
                        .find(|n| n.kind == DeviceKind::Router)
                        .map(|n| n.ip)
                        .unwrap_or_else(|| session.vantage_ip());
                    let hops = traceroute(&graph, session.vantage_ip(), ip, router_ip);
                    if hops.is_empty() {
                        println!("no route");
                    } else {
                        for hop in hops {
                            println!("  {}  {}  {}ms", hop.ip, hop.name, hop.latency_ms);
                        }
                    }
                }
                None => println!("could not resolve {arg}"),
            },
            "" => {}
            other => println!("unknown command: {other}"),
        }
        if session.traced() {
            println!("*** TRACED - the intrusion was traced. ***");
            break;
        }
    }
    Ok(())
}
