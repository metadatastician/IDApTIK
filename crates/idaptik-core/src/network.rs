//! The network graph: devices, the links between them, and the reachability
//! queries the hacker's tooling is built on (traceroute-style hop paths, and
//! "what can I see from this foothold").

use crate::device::{Device, DeviceId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

//## Network segmentation categories

/// Network segmentation category. The concrete network segments live as data in
/// the netsim graph; this is the coarse category each segment falls under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    Lan,
    Dmz,
    Internal,
    Iot,
    Management,
    Scada,
    Isp,
    Service,
}

/// How far a segment sits from the target the infiltrator is inside. One network
/// runs from the wider internet down to the corridor; `Range` places each segment
/// along that distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Range {
    WideArea,
    Perimeter,
    LocalLan,
}

/// An undirected graph of devices and the links between them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Network {
    devices: HashMap<DeviceId, Device>,
    links: HashMap<DeviceId, HashSet<DeviceId>>,
}

impl Network {
    /// An empty network.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a device. Idempotent on its id; re-adding replaces the device but
    /// keeps its links.
    pub fn add_device(&mut self, device: Device) {
        let id = device.id;
        self.devices.insert(id, device);
        self.links.entry(id).or_default();
    }

    /// Link two devices (undirected). Missing endpoints get a links entry so the
    /// graph stays consistent even if a device is added later.
    pub fn link(&mut self, a: DeviceId, b: DeviceId) {
        self.links.entry(a).or_default().insert(b);
        self.links.entry(b).or_default().insert(a);
    }

    /// Look up a device by id.
    pub fn device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.get(&id)
    }

    /// Iterate every device.
    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }

    /// Number of devices.
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// Whether the network has no devices.
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// The devices directly linked to `id`.
    pub fn neighbors(&self, id: DeviceId) -> impl Iterator<Item = DeviceId> + '_ {
        self.links
            .get(&id)
            .into_iter()
            .flat_map(|set| set.iter().copied())
    }

    /// Shortest hop path from `from` to `to` (BFS), inclusive of both ends —
    /// the traceroute the hacker sees. `None` if unreachable.
    pub fn hop_path(&self, from: DeviceId, to: DeviceId) -> Option<Vec<DeviceId>> {
        if !self.devices.contains_key(&from) || !self.devices.contains_key(&to) {
            return None;
        }
        if from == to {
            return Some(vec![from]);
        }
        let mut prev: HashMap<DeviceId, DeviceId> = HashMap::new();
        let mut seen: HashSet<DeviceId> = HashSet::from([from]);
        let mut queue: VecDeque<DeviceId> = VecDeque::from([from]);

        while let Some(node) = queue.pop_front() {
            for next in self.neighbors(node) {
                if seen.insert(next) {
                    prev.insert(next, node);
                    if next == to {
                        // Reconstruct the path from `to` back to `from`.
                        let mut path = vec![to];
                        let mut cur = to;
                        while let Some(&p) = prev.get(&cur) {
                            path.push(p);
                            cur = p;
                        }
                        path.reverse();
                        return Some(path);
                    }
                    queue.push_back(next);
                }
            }
        }
        None
    }

    /// Every device reachable from `start` (including `start`) — what the hacker
    /// can discover from a given foothold.
    pub fn reachable_from(&self, start: DeviceId) -> HashSet<DeviceId> {
        let mut seen = HashSet::new();
        if !self.devices.contains_key(&start) {
            return seen;
        }
        seen.insert(start);
        let mut queue = VecDeque::from([start]);
        while let Some(node) = queue.pop_front() {
            for next in self.neighbors(node) {
                if seen.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceKind, SecurityLevel};
    use std::net::Ipv4Addr;

    fn dev(id: u32, zone: Zone) -> Device {
        Device::new(
            DeviceId(id),
            format!("d{id}"),
            Ipv4Addr::new(10, 0, 0, id as u8),
            DeviceKind::Server,
            SecurityLevel::Weak,
            zone,
        )
    }

    #[test]
    fn hop_path_finds_shortest_route() {
        let mut net = Network::new();
        for id in 0..4 {
            net.add_device(dev(id, Zone::Internal));
        }
        // 0-1-2 chain, plus a direct 0-3 and 3-2 shortcut of equal length.
        net.link(DeviceId(0), DeviceId(1));
        net.link(DeviceId(1), DeviceId(2));
        let path = net.hop_path(DeviceId(0), DeviceId(2)).unwrap();
        assert_eq!(path.first(), Some(&DeviceId(0)));
        assert_eq!(path.last(), Some(&DeviceId(2)));
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn unreachable_returns_none() {
        let mut net = Network::new();
        net.add_device(dev(0, Zone::Dmz));
        net.add_device(dev(1, Zone::Internal)); // island, no link
        assert!(net.hop_path(DeviceId(0), DeviceId(1)).is_none());
        assert_eq!(
            net.reachable_from(DeviceId(0)),
            HashSet::from([DeviceId(0)])
        );
    }
}
