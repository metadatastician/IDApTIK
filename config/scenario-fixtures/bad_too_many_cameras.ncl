# Negative fixture: 226 hallway cameras — one more than the security subnet
# can address (floor_graph::MAX_CAMERAS = 254 - 30 + 1 = 225). The scenario
# contract must reject this, mirroring `ValidationError::TooManyCameras`.
# `just config-check` asserts `nickel export` exits non-zero here.
let scenario = import "../scenario-schema.ncl" in
let good = import "../ghost_lobby_scenario.ncl" in
(
  good
  & {
    cameras
      | force
      = std.array.generate
        (fun i =>
          {
            id = "cam-%{std.string.from_number i}",
            room = "hall",
            x = 330.0,
            range = [292.0, 495.0],
            phase = 0.2,
            stale = false,
          })
        226,
  }
) | scenario.ScenarioDefinition
