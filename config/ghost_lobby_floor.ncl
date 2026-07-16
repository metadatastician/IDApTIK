let schema = import "schema.ncl" in
{
  segments = [
    { id = "perimeter-wifi", range = 'Perimeter, category = 'Iot, subnet = "10.20.9.", can_access = ["bms", "isp"] },
    { id = "bms", range = 'Perimeter, category = 'Management, subnet = "10.20.5.", can_access = ["auto-kitchen", "auto-hall", "auto-office", "auto-laundry", "auto-exit", "security", "perimeter-wifi"] },
    { id = "auto-kitchen", range = 'LocalLan, category = 'Iot, subnet = "10.20.10.", can_access = ["bms"], location = "kitchen" },
    { id = "auto-hall", range = 'LocalLan, category = 'Iot, subnet = "10.20.11.", can_access = ["bms"], location = "hall" },
    { id = "auto-office", range = 'LocalLan, category = 'Iot, subnet = "10.20.12.", can_access = ["bms"], location = "office" },
    { id = "auto-laundry", range = 'LocalLan, category = 'Iot, subnet = "10.20.13.", can_access = ["bms"], location = "laundry" },
    { id = "auto-exit", range = 'LocalLan, category = 'Iot, subnet = "10.20.14.", can_access = ["bms"], location = "exit" },
    { id = "security", range = 'LocalLan, category = 'Iot, subnet = "10.20.4.", can_access = ["bms"] },
    { id = "isp", range = 'WideArea, category = 'Isp, subnet = "100.64.0.", can_access = ["utility-dmz"] },
    { id = "utility-dmz", range = 'WideArea, category = 'Dmz, subnet = "10.10.0.", can_access = ["utility"] },
    { id = "utility", range = 'WideArea, category = 'Scada, subnet = "10.10.1.", can_access = [] },
  ],
  nodes = [
    { id = "substation", name = "GRID SUBSTATION", ip = "10.10.1.5", segment = "utility", kind = 'Substation, security = 'Strong, actuation = "CutPower" },
    { id = "isp-gw", name = "ISP GATEWAY", ip = "100.64.0.1", segment = "isp", kind = 'Router, security = 'Medium },
    { id = "isp-ops", name = "ISP OPS HOST", ip = "100.64.0.9", segment = "isp", kind = 'Server, security = 'Strong },
    { id = "grid-jump", name = "GRID JUMP HOST", ip = "10.10.0.9", segment = "utility-dmz", kind = 'Server, security = 'Strong },
    { id = "ap-lobby", name = "LOBBY WIFI AP", ip = "10.20.9.1", segment = "perimeter-wifi", kind = 'Router, security = 'Weak },
    { id = "bms-bridge", name = "MAINT BRIDGE", ip = "10.20.5.2", segment = "bms", kind = 'Server, security = 'Medium },
    { id = "feed", name = "BUILDING FEED", ip = "10.20.5.3", segment = "bms", kind = 'PowerStation, security = 'Medium, deps = [{ on = "substation" }] },
  ],
  dns = [
    { host = "bridge.local", ip = "10.20.5.2" },
    { host = "ap.local", ip = "10.20.9.1" },
    { host = "ops.isp.net", ip = "100.64.0.9" },
    { host = "jump.grid.local", ip = "10.10.0.9" },
    { host = "substation.grid.local", ip = "10.10.1.5" },
  ],
  vantages = [
    { kind = "Van", entry_ip = "10.20.9.1", physical_risk = 40 },
  ],
} | schema.GroundedGraph
