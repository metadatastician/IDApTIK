# Negative fixture: door D2 renamed to "D1", duplicating an id. The scenario
# contract must reject this, mirroring `ValidationError::DuplicateDoorId`.
# `just config-check` asserts `nickel export` exits non-zero here.
let scenario = import "../scenario-schema.ncl" in
let good = import "../ghost_lobby_scenario.ncl" in
(
  good
  & {
    doors
      | force
      = good.doors
      |> std.array.map (fun d => if d.id == "D2" then d & { id | force = "D1" } else d),
  }
) | scenario.ScenarioDefinition
