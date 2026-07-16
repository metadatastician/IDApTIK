//! The vertical-slice acceptance: two vantages, and two winning lines to pass
//! one door (hack it locally, or black out the upstream power so its maglock
//! fails), plus the genuine pivot the van must perform to reach the building.
use idaptik_core::netsim::{
    AgentSession, Effect, GroundedGraph, VantageDef, VantageKind, grounded_slice,
};
use std::net::Ipv4Addr;

fn vantage(g: &GroundedGraph, kind: VantageKind) -> VantageDef {
    g.vantages
        .iter()
        .find(|v| v.kind == kind)
        .cloned()
        .expect("vantage present")
}

#[test]
fn inside_vantage_opens_the_door_locally() {
    let g = grounded_slice();
    let v = vantage(&g, VantageKind::Inside);
    let mut s = AgentSession::new(&g, v, 1000);
    let effects = s
        .hack(&g, "door-hall-office")
        .expect("door reachable from inside");
    assert!(effects.contains(&Effect::DoorHeld("door-hall-office".into())));
    assert!(!s.traced());
}

#[test]
fn wide_area_power_line_disables_the_maglock_door() {
    let g = grounded_slice();
    let v = vantage(&g, VantageKind::Base);
    let mut s = AgentSession::new(&g, v, 1000);
    // From the remote base the substation is reachable over the ISP hop; hacking it
    // cascades power loss down to the building's maglock door and lights.
    let effects = s
        .hack(&g, "substation")
        .expect("substation reachable from the ISP foothold");
    assert!(effects.contains(&Effect::PowerCut("substation".into())));
    assert!(effects.contains(&Effect::DevicePowerLost("door-hall-office".into())));
    assert!(effects.contains(&Effect::DevicePowerLost("light-hall".into())));
}

#[test]
fn identical_command_sequences_are_deterministic() {
    let g = grounded_slice();
    let run = || {
        let v = vantage(&g, VantageKind::Van);
        let mut s = AgentSession::new(&g, v, 1000);
        let pivot = s.ssh(&g, "bridge.local");
        let hack = s.hack(&g, "door-hall-office");
        (pivot, hack, s.logs().to_vec(), s.trace_fraction())
    };
    assert_eq!(run(), run());
}

#[test]
fn van_must_pivot_through_the_bridge_to_reach_the_building() {
    let g = grounded_slice();
    let v = vantage(&g, VantageKind::Van);
    let mut s = AgentSession::new(&g, v, 1000);
    let door = Ipv4Addr::new(10, 20, 0, 12);
    let bridge = Ipv4Addr::new(10, 20, 5, 2);
    // From the van's perimeter foothold the building door is not directly reachable;
    // only the bridge host is.
    assert!(
        !s.reachable(&g).contains(&door),
        "door must not be reachable before pivoting"
    );
    assert!(
        s.reachable(&g).contains(&bridge),
        "the bridge host is the way in"
    );
    // Pivot through the bridge host, and the local devices open up.
    s.ssh(&g, "bridge.local")
        .expect("bridge host is reachable and pivotable");
    assert!(
        s.reachable(&g).contains(&door),
        "door becomes reachable after pivoting to the bridge"
    );
}
