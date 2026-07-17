# Negative fixture: kitchen support pushed above 1.0. The scenario contract
# must reject this, mirroring `ValidationError::SupportOutOfRange`.
# `just config-check` asserts `nickel export` exits non-zero here.
let scenario = import "../scenario-schema.ncl" in
let good = import "../ghost_lobby_scenario.ncl" in
(
  good
  & {
    rooms
      | force
      = good.rooms
      |> std.array.map (fun r => if r.id == "kitchen" then r & { support | force = 1.5 } else r),
  }
) | scenario.ScenarioDefinition
