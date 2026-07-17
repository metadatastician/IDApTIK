# Contracts for IDApTIK configuration.
#
# These mirror the domain types in `crates/idaptik-core` so that level/network
# config is validated before it ever reaches the engine. Concrete configs import
# this and apply `schema.Network` (see default.ncl).
{
  DeviceKind
    | doc "The kind of device on the network."
    = [| 'Laptop, 'Router, 'Server, 'IotCamera, 'Terminal, 'PowerStation, 'Ups,
         'Firewall, 'SmartDoor, 'Camera, 'Lock, 'Elevator, 'Light, 'Sensor,
         'Substation |],

  SecurityLevel
    | doc "How hard a device is to compromise."
    = [| 'Open, 'Weak, 'Medium, 'Strong |],

  Zone
    | doc "Network segmentation category."
    = [| 'Lan, 'Dmz, 'Internal, 'Iot, 'Management, 'Scada, 'Isp, 'Service |],

  Range
    | doc "How far a segment sits from the target."
    = [| 'WideArea, 'Perimeter, 'LocalLan |],

  Actuation
    | doc "What actuating a node does to the world."
    = [| 'HoldDoor, 'DisengageLock, 'LoopCamera, 'DisableCamera, 'CallElevator,
         'KillLights, 'CutPower, 'MuteSensor, 'RunVacuum |],

  VantageKind
    | doc "Where an agent physically plays from."
    = [| 'Base, 'InternetCafe, 'Van, 'Inside |],

  Device = {
    name | String,
    ip | String,
    kind | DeviceKind,
    security | SecurityLevel,
    zone | Zone,
  },

  # A link is a pair of device names that are connected (undirected).
  Link = Array String,

  Network = {
    devices | Array Device,
    links | Array Link | default = [],
  },

  Segment
    | doc "A concrete network segment in the grounded graph."
    = {
      id | String,
      range | Range,
      category | Zone,
      subnet | String,
      can_access | Array String | default = [],
      location | String | optional,
    },

  Node
    | doc "A device or system on the grounded network."
    = {
      id | String,
      name | String,
      ip | String,
      segment | String,
      kind | DeviceKind,
      security | SecurityLevel,
      actuation | Actuation | optional,
      deps | Array { on | String } | default = [],
    },

  DnsRecord
    | doc "One authoritative DNS record."
    = { host | String, ip | String },

  Vantage
    | doc "Where the hacker physically is: an entry node and a physical-discovery risk."
    = { kind | VantageKind, entry_ip | String, physical_risk | Number },

  GroundedGraph
    | doc "The whole grounded network as data."
    = {
      segments | Array Segment,
      nodes | Array Node,
      dns | Array DnsRecord | default = [],
      vantages | Array Vantage | default = [],
    },
}
