# Negative fixture: door D1 dragged off its room boundary (270 -> 300, inside
# the hallway span). The scenario contract must reject this, mirroring
# `ValidationError::DoorOffRoomBoundary`.
# `just config-check` asserts `nickel export` exits non-zero here.
let scenario = import "../scenario-schema.ncl" in
let good = import "../ghost_lobby_scenario.ncl" in
(
  good
  & {
    doors
      | force
      = good.doors
      |> std.array.map (fun d => if d.id == "D1" then d & { x | force = 300.0 } else d),
  }
) | scenario.ScenarioDefinition
