# A starter network, validated against schema.ncl. Mirrors `demo_network()` in
# crates/idaptik-core. Check it with `just config-check` (or `nickel export`).
let schema = import "schema.ncl" in
{
  devices = [
    { name = "edge-router", ip = "10.0.0.1", kind = 'Router, security = 'Medium, zone = 'Dmz },
    { name = "web", ip = "10.0.0.10", kind = 'Server, security = 'Weak, zone = 'Dmz },
    { name = "db", ip = "10.0.1.10", kind = 'Server, security = 'Strong, zone = 'Internal },
    { name = "ops-terminal", ip = "10.0.1.20", kind = 'Terminal, security = 'Medium, zone = 'Internal },
    { name = "lobby-cam", ip = "192.168.100.5", kind = 'IotCamera, security = 'Open, zone = 'Iot },
    { name = "ups", ip = "10.10.0.2", kind = 'Ups, security = 'Weak, zone = 'Scada },
  ],
  links = [
    ["edge-router", "web"],
    ["web", "db"],
    ["db", "ops-terminal"],
    ["edge-router", "lobby-cam"],
    ["lobby-cam", "ups"],
  ],
} | schema.Network
