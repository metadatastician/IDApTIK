let schema = import "schema.ncl" in
{
  segments = [
    { id = "local-auto", range = 'LocalLan, category = 'Internal, subnet = "10.20.0.", can_access = [], location = "building" },
    { id = "bridge", range = 'Perimeter, category = 'Dmz, subnet = "10.20.5.", can_access = ["local-auto"] },
    { id = "perimeter-wifi", range = 'Perimeter, category = 'Iot, subnet = "10.20.9.", can_access = ["bridge"] },
    { id = "isp", range = 'WideArea, category = 'Isp, subnet = "100.64.0.", can_access = ["utility"] },
    { id = "utility", range = 'WideArea, category = 'Scada, subnet = "10.10.1.", can_access = [] },
  ],
  nodes = [
    { id = "substation", name = "GRID SUBSTATION", ip = "10.10.1.5", segment = "utility", kind = 'Substation, security = 'Strong, actuation = "CutPower" },
    { id = "feed", name = "BUILDING FEED", ip = "10.20.0.2", segment = "local-auto", kind = 'PowerStation, security = 'Medium, deps = [{ on = "substation" }] },
    { id = "door-hall-office", name = "HALL/OFFICE DOOR", ip = "10.20.0.12", segment = "local-auto", kind = 'SmartDoor, security = 'Weak, actuation = "HoldDoor", deps = [{ on = "feed" }] },
    { id = "light-hall", name = "HALL LIGHTS", ip = "10.20.0.20", segment = "local-auto", kind = 'Light, security = 'Weak, actuation = "KillLights", deps = [{ on = "feed" }] },
    { id = "camera-hall", name = "HALL CAMERA", ip = "10.20.0.30", segment = "local-auto", kind = 'Camera, security = 'Weak, actuation = "LoopCamera", deps = [{ on = "feed" }] },
    { id = "bridge-host", name = "MAINT BRIDGE", ip = "10.20.5.2", segment = "bridge", kind = 'Server, security = 'Medium },
    { id = "isp-gw", name = "ISP GATEWAY", ip = "100.64.0.1", segment = "isp", kind = 'Router, security = 'Medium },
    { id = "ap-lobby", name = "LOBBY WIFI AP", ip = "10.20.9.1", segment = "perimeter-wifi", kind = 'Router, security = 'Weak },
  ],
  dns = [
    { host = "door.hall.local", ip = "10.20.0.12" },
    { host = "bridge.local", ip = "10.20.5.2" },
    { host = "substation.grid.local", ip = "10.10.1.5" },
  ],
  vantages = [
    { kind = "Base", entry_ip = "100.64.0.1", physical_risk = 0 },
    { kind = "Inside", entry_ip = "10.20.0.30", physical_risk = 85 },
    { kind = "Van", entry_ip = "10.20.9.1", physical_risk = 40 },
  ],
} | schema.GroundedGraph
