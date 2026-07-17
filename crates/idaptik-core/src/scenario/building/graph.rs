//! The building's physical graph: rooms are nodes, portals are weighted,
//! bidirectional edges. Deterministic reachability and shortest paths —
//! `BTreeMap`/`BTreeSet` throughout, so iteration order can never leak into
//! results, and no panicking paths.

use crate::scenario::building::{BuildingDefinition, PortalDef, RoomRef};
use crate::scenario::ids::PortalId;
use std::collections::{BTreeMap, BTreeSet};

/// A shortest route through the building.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The rooms visited, starting at `from` and ending at `to`.
    pub rooms: Vec<RoomRef>,
    /// The portals traversed, one fewer than `rooms`.
    pub portals: Vec<PortalId>,
    /// Total travel time in seconds.
    pub travel_time: f64,
}

/// The rooms reachable from `from` through portals `passable` admits,
/// including `from` itself (if it exists). Returned in sorted order.
pub fn reachable_rooms(
    def: &BuildingDefinition,
    from: &RoomRef,
    passable: impl Fn(&PortalDef) -> bool,
) -> BTreeSet<RoomRef> {
    let mut seen: BTreeSet<RoomRef> = BTreeSet::new();
    if !def.has_room(from) {
        return seen;
    }
    seen.insert(from.clone());
    let mut frontier: Vec<RoomRef> = vec![from.clone()];
    while let Some(here) = frontier.pop() {
        for p in &def.portals {
            if !passable(p) {
                continue;
            }
            let next = if p.from == here {
                &p.to
            } else if p.to == here {
                &p.from
            } else {
                continue;
            };
            if def.has_room(next) && seen.insert(next.clone()) {
                frontier.push(next.clone());
            }
        }
    }
    seen
}

/// The shortest route (by total travel time) from `from` to `to` through
/// portals `passable` admits, or `None` if unreachable. Deterministic: ties
/// break on room order, so equal-cost graphs always yield the same route.
pub fn shortest_path(
    def: &BuildingDefinition,
    from: &RoomRef,
    to: &RoomRef,
    passable: impl Fn(&PortalDef) -> bool,
) -> Option<Route> {
    if !def.has_room(from) || !def.has_room(to) {
        return None;
    }
    let mut dist: BTreeMap<RoomRef, f64> = BTreeMap::new();
    let mut prev: BTreeMap<RoomRef, (RoomRef, PortalId)> = BTreeMap::new();
    let mut done: BTreeSet<RoomRef> = BTreeSet::new();
    dist.insert(from.clone(), 0.0);

    loop {
        // The unvisited room with the least distance; BTreeMap order breaks ties.
        let here = dist
            .iter()
            .filter(|(room, _)| !done.contains(*room))
            .fold(None::<(&RoomRef, f64)>, |best, (room, &d)| match best {
                Some((_, bd)) if bd.total_cmp(&d) != std::cmp::Ordering::Greater => best,
                _ => Some((room, d)),
            })
            .map(|(room, _)| room.clone());
        let Some(here) = here else {
            return None; // exhausted without reaching `to`
        };
        if &here == to {
            break;
        }
        done.insert(here.clone());
        let here_dist = dist.get(&here).copied().unwrap_or(f64::INFINITY);
        for p in &def.portals {
            if !passable(p) {
                continue;
            }
            let next = if p.from == here {
                &p.to
            } else if p.to == here {
                &p.from
            } else {
                continue;
            };
            if done.contains(next) || !def.has_room(next) {
                continue;
            }
            let candidate = here_dist + p.travel_time.max(0.0);
            let better = dist
                .get(next)
                .is_none_or(|&d| candidate.total_cmp(&d) == std::cmp::Ordering::Less);
            if better {
                dist.insert(next.clone(), candidate);
                prev.insert(next.clone(), (here.clone(), p.id.clone()));
            }
        }
    }

    // Unwind the predecessor chain. Bounded by the room count, so a cycle in
    // `prev` (impossible by construction) could not loop forever.
    let mut rooms: Vec<RoomRef> = vec![to.clone()];
    let mut portals: Vec<PortalId> = Vec::new();
    let mut cursor = to.clone();
    for _ in 0..=def.all_rooms().len() {
        if &cursor == from {
            break;
        }
        let (back, via) = prev.get(&cursor)?;
        rooms.push(back.clone());
        portals.push(via.clone());
        cursor = back.clone();
    }
    rooms.reverse();
    portals.reverse();
    Some(Route {
        rooms,
        portals,
        travel_time: dist.get(to).copied().unwrap_or(0.0),
    })
}
