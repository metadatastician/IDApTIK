# Contracts for IDApTIK configuration.
#
# These mirror the domain types in `crates/idaptik-core` so that level/network
# config is validated before it ever reaches the engine. Concrete configs import
# this and apply `schema.Network` (see default.ncl).
{
  DeviceKind
    | doc "The kind of device on the network."
    = [| 'Laptop, 'Router, 'Server, 'IotCamera, 'Terminal, 'PowerStation, 'Ups |],

  SecurityLevel
    | doc "How hard a device is to compromise."
    = [| 'Open, 'Weak, 'Medium, 'Strong |],

  Zone
    | doc "Network segmentation zone."
    = [| 'Dmz, 'Internal, 'Scada, 'Iot, 'External |],

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
}
