//! The embedded slice graph: the Ghost Lobby building plus a perimeter and a
//! wide-area power line, authored in config/grounded_slice.ncl.
use crate::netsim::graph::GroundedGraph;

/// The committed, pretty-printed slice graph. Regenerate with the ignored
/// `regenerate_slice_json` test after changing config/grounded_slice.ncl.
pub const GROUNDED_SLICE_JSON: &str = include_str!("grounded_slice.json");

/// Deserialise the embedded slice graph.
pub fn grounded_slice() -> GroundedGraph {
    serde_json::from_str(GROUNDED_SLICE_JSON).expect("embedded grounded_slice.json is valid")
}

#[cfg(test)]
mod regen {
    use super::*;

    /// Rewrites the committed golden from the Nickel export. Run once to seed it:
    /// `cargo test -p idaptik-core regenerate_slice_json -- --ignored`.
    #[test]
    #[ignore = "regenerates the committed golden; run manually"]
    fn regenerate_slice_json() {
        let raw = std::fs::read_to_string("/tmp/grounded_slice_raw.json").expect("nickel export");
        let g: GroundedGraph = serde_json::from_str(&raw).expect("parse nickel json");
        let mut pretty = serde_json::to_string_pretty(&g).expect("serialize");
        pretty.push('\n');
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/netsim/grounded_slice.json"
        );
        std::fs::write(path, pretty.as_bytes()).expect("write golden");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netsim::effect::{Effect, apply_actuation};

    #[test]
    fn slice_loads_and_the_door_depends_on_the_substation() {
        let g = grounded_slice();
        assert!(g.node("door-hall-office").is_some());
        assert!(g.node("substation").is_some());
        // Cutting the substation must take the door's power (the wide-area line).
        let effects = apply_actuation(&g, "substation");
        assert!(effects.contains(&Effect::DevicePowerLost("door-hall-office".into())));
    }

    #[test]
    fn committed_json_is_serde_round_trip_stable() {
        // Guards serde-canonical stability of the committed golden: it must parse
        // to a graph that re-serialises identically. It does NOT check freshness
        // against config/grounded_slice.ncl; regenerate after editing the Nickel.
        let g = grounded_slice();
        let round = serde_json::to_string_pretty(&g).expect("serialize");
        assert_eq!(round.trim(), GROUNDED_SLICE_JSON.trim());
    }
}
