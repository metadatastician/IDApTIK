# Contracts for the declarative scenario definition (ADR-0001, issue #10).
#
# These mirror `crates/idaptik-core/src/scenario/definition.rs` — the record
# shapes — and `ScenarioDefinition::validate` — the semantic checks — so a
# level authored as typed config is validated before it ever reaches the
# engine. Concrete scenarios import this and apply `ScenarioDefinition` (see
# ghost_lobby_scenario.ncl); each semantic validator names the Rust
# `ValidationError` variant it mirrors in its blame message.
#
# Deliberate strictness deltas versus serde + `validate()`:
#   - Records are closed: unknown fields are rejected, whereas serde would
#     silently ignore them. This catches typos at authoring time.
#   - `format` is a free string, exactly as in `validate()` (the
#     "idaptik-ghost-lobby-v2" tag is asserted by the loaders, not the
#     validator).

# Float boundary tolerance — mirrors `definition::EPS`.
let eps = 0.000001 in

# Capacity caps — mirror `floor_graph::MAX_CAMERAS` (254 - 30 + 1) and
# `floor_graph::MAX_DOORS_PER_ROOM` (254 - 10). Keep in lock-step with
# `crates/idaptik-core/src/scenario/floor_graph.rs`.
let max_cameras = 225 in
let max_doors_per_room = 244 in

let abs = fun x => if x < 0 then -x else x in

# Whether an array of strings contains a duplicate.
let has_duplicate = fun ids =>
  let sorted = std.array.sort (fun a b => std.string.compare a b) ids in
  let n = std.array.length sorted in
  if n < 2 then
    false
  else
    std.array.generate (fun i => std.array.at i sorted == std.array.at (i + 1) sorted) (n - 1)
    |> std.array.any (fun d => d)
in

# Mirrors `GhostLobbySim::room_index_at`: the first room whose `[x, x + w)`
# span contains `x + room_offset`, else the last room's index.
let room_index_at = fun rooms offset x =>
  let cx = x + offset in
  let n = std.array.length rooms in
  let hit =
    std.array.generate (fun i => i) n
    |> std.array.fold_left
      (fun acc i =>
        if acc != null then
          acc
        else
          let r = std.array.at i rooms in
          if cx >= r.x && cx < r.x + r.w then i else null)
      null
  in
  if hit == null then n - 1 else hit
in

# Mirrors `ScenarioDefinition::is_on_boundary`.
let on_boundary = fun rooms x =>
  rooms |> std.array.any (fun r => abs (r.x - x) <= eps || abs (r.x + r.w - x) <= eps)
in

# Leftmost room `x` / rightmost room `x + w` — mirrors `world_bounds`.
let world_left = fun rooms =>
  rooms |> std.array.fold_left (fun acc r => if r.x < acc then r.x else acc) (std.array.first rooms).x
in
let world_right = fun rooms =>
  rooms |> std.array.fold_left (fun acc r => if r.x + r.w > acc then r.x + r.w else acc) 0
in

# A Rust `(f64, f64)` tuple: a length-2 array of numbers.
let Pair =
  std.contract.from_validator (fun value =>
    if !std.is_array value then
      'Error { message = "expected a [base, span] pair (array of 2 numbers)" }
    else if std.array.length value != 2 then
      'Error { message = "expected exactly 2 elements, got %{std.string.from_number (std.array.length value)}" }
    else if !(value |> std.array.all (fun v => std.is_number v)) then
      'Error { message = "pair elements must be numbers" }
    else
      'Ok
  )
in

# A pair with `fst <= snd` — mirrors the `ArrivalInverted` check.
let OrderedPair =
  std.contract.Sequence [
    Pair,
    std.contract.from_validator (fun value =>
      if std.array.first value <= std.array.last value then
        'Ok
      else
        'Error { message = "ArrivalInverted: pair must be ordered (fst <= snd)" }
    ),
  ]
in

# A pair with `fst < snd` — mirrors the `CameraRangeInverted` check.
let SweepRange =
  std.contract.Sequence [
    Pair,
    std.contract.from_validator (fun value =>
      if std.array.first value < std.array.last value then
        'Ok
      else
        'Error { message = "CameraRangeInverted: sweep range must satisfy fst < snd" }
    ),
  ]
in

# A Rust `u32`.
let U32 =
  std.contract.from_validator (fun value =>
    if !std.is_number value then
      'Error { message = "expected a number" }
    else if !std.number.is_integer value || value < 0 || value > 4294967295 then
      'Error { message = "expected a u32 (integer in [0, 4294967295])" }
    else
      'Ok
  )
in

# Room support envelope — mirrors the `SupportOutOfRange` check.
let Support =
  std.contract.from_validator (fun value =>
    if !std.is_number value then
      'Error { message = "expected a number" }
    else if value < 0.05 || value > 1.0 then
      'Error { message = "SupportOutOfRange: room support must be within [0.05, 1.0]" }
    else
      'Ok
  )
in

# Non-negative number — mirrors the `NegativeActionCost` check.
let NonNegative =
  std.contract.from_validator (fun value =>
    if !std.is_number value then
      'Error { message = "expected a number" }
    else if value < 0 then
      'Error { message = "NegativeActionCost: action cost/cooldown must be non-negative" }
    else
      'Ok
  )
in

# `Option<T>`: `null` (serde `None`) or a value satisfying `contract`.
let Nullable = fun contract =>
  std.contract.custom (fun label value =>
    if value == null then
      'Ok value
    else
      std.contract.check contract label value
  )
in

# --- Whole-definition semantic checks (mirroring `validate()`) -----------------

# EmptyRooms
let rooms_present =
  std.contract.from_validator (fun def =>
    if std.array.length def.rooms > 0 then
      'Ok
    else
      'Error { message = "EmptyRooms: at least one room must be defined" }
  )
in

# DuplicateRoomId
let unique_room_ids =
  std.contract.from_validator (fun def =>
    if has_duplicate (def.rooms |> std.array.map (fun r => r.id)) then
      'Error { message = "DuplicateRoomId: room ids must be unique" }
    else
      'Ok
  )
in

# DuplicateDoorId
let unique_door_ids =
  std.contract.from_validator (fun def =>
    if has_duplicate (def.doors |> std.array.map (fun d => d.id)) then
      'Error { message = "DuplicateDoorId: door ids must be unique" }
    else
      'Ok
  )
in

# RoomsNotContiguous: room[i].x + room[i].w == room[i+1].x (within eps).
let rooms_contiguous =
  std.contract.from_validator (fun def =>
    let rooms = def.rooms in
    let n = std.array.length rooms in
    let gap =
      if n < 2 then
        false
      else
        std.array.generate
          (fun i =>
            let a = std.array.at i rooms in
            let b = std.array.at (i + 1) rooms in
            abs ((a.x + a.w) - b.x) > eps)
          (n - 1)
        |> std.array.any (fun g => g)
    in
    if gap then
      'Error { message = "RoomsNotContiguous: rooms must tile the world with no gaps or overlaps" }
    else
      'Ok
  )
in

# DoorOffRoomBoundary
let doors_on_boundaries =
  std.contract.from_validator (fun def =>
    if def.doors |> std.array.all (fun d => on_boundary def.rooms d.x) then
      'Ok
    else
      'Error { message = "DoorOffRoomBoundary: every door must sit on a room boundary" }
  )
in

# HideSpotRoomMissing
let hide_spot_rooms_exist =
  std.contract.from_validator (fun def =>
    let room_ids = def.rooms |> std.array.map (fun r => r.id) in
    if def.hide_spots |> std.array.all (fun h => std.array.elem h.room room_ids) then
      'Ok
    else
      'Error { message = "HideSpotRoomMissing: every hide spot must reference a real room" }
  )
in

# CameraRoomMissing
let camera_rooms_exist =
  std.contract.from_validator (fun def =>
    let room_ids = def.rooms |> std.array.map (fun r => r.id) in
    if def.cameras |> std.array.all (fun c => std.array.elem c.room room_ids) then
      'Ok
    else
      'Error { message = "CameraRoomMissing: every camera must reference a real room" }
  )
in

# ObjectiveRoomMissing (a `null` room is fine — it is serde's `None`).
let objective_rooms_exist =
  std.contract.from_validator (fun def =>
    let room_ids = def.rooms |> std.array.map (fun r => r.id) in
    if def.objectives |> std.array.all (fun o => o.room == null || std.array.elem o.room room_ids) then
      'Ok
    else
      'Error { message = "ObjectiveRoomMissing: every objective room reference must resolve" }
  )
in

# PlayerSpawnOutOfBounds
let player_spawn_in_bounds =
  std.contract.from_validator (fun def =>
    let x = def.player.spawn_x in
    if x >= world_left def.rooms && x <= world_right def.rooms then
      'Ok
    else
      'Error { message = "PlayerSpawnOutOfBounds: the player must spawn within the world" }
  )
in

# ObjectSpawnOutOfBounds (note / usb static spawn bases, as in `validate()`).
let prop_spawns_in_bounds =
  std.contract.from_validator (fun def =>
    let left = world_left def.rooms in
    let right = world_right def.rooms in
    if [def.props.note.x, def.props.usb.x] |> std.array.all (fun x => x >= left && x <= right) then
      'Ok
    else
      'Error { message = "ObjectSpawnOutOfBounds: note/usb spawn bases must be within the world" }
  )
in

# NoExtractionRoom
let extraction_objective_exists =
  std.contract.from_validator (fun def =>
    if def.objectives |> std.array.any (fun o => o.kind == 'Exit) then
      'Ok
    else
      'Error { message = "NoExtractionRoom: at least one Exit objective must exist" }
  )
in

# TooManyCameras
let cameras_fit_subnet =
  std.contract.from_validator (fun def =>
    let count = std.array.length def.cameras in
    if count <= max_cameras then
      'Ok
    else
      'Error {
        message = "TooManyCameras: %{std.string.from_number count} camera(s) overflow the security subnet (max %{std.string.from_number max_cameras})",
      }
  )
in

# TooManyDoorsInRoom
let doors_fit_room_subnets =
  std.contract.from_validator (fun def =>
    let rooms = def.rooms in
    let per_room =
      std.array.generate
        (fun i =>
          def.doors
          |> std.array.filter (fun d => room_index_at rooms def.world.room_offset d.x == i)
          |> std.array.length)
        (std.array.length rooms)
    in
    if per_room |> std.array.all (fun n => n <= max_doors_per_room) then
      'Ok
    else
      'Error {
        message = "TooManyDoorsInRoom: a room's doors overflow its automation subnet (max %{std.string.from_number max_doors_per_room})",
      }
  )
in

{
  ObjectiveKind
    | doc "Objective category."
    = [| 'Note, 'Misdirect, 'Exit |],

  World
    | doc "World-level constants: the floor line and overall extent."
    = { floor | Number, room_offset | Number, width | Number },

  Room
    | doc "A room: an [x, x + w) span with per-room support / sight quirks."
    = {
      id | String,
      name | String,
      x | Number,
      w | Number,
      support | Support,
      ping_support_bonus | Number,
      sight_multiplier | Number,
      lit | Bool,
    },

  Door
    | doc "A door on a room boundary."
    = { id | String, x | Number, label | String },

  HideSpot
    | doc "A hide spot: crouch within `radius` of `x` in `room`."
    = { id | String, room | String, x | Number, radius | Number, label | String },

  Camera
    | doc "A patrol camera with a sinusoidal sweep over [range.0, range.1]."
    = {
      id | String,
      room | String,
      x | Number,
      range | SweepRange,
      phase | Number,
      stale | Bool,
    },

  PropSpawn
    | doc "A static prop spawn position."
    = { x | Number, y | Number },

  Props
    | doc "The four interactive props."
    = { note | PropSpawn, usb | PropSpawn, chute | PropSpawn, vacuum | PropSpawn },

  Body
    | doc "A spawn position and body box (player / Billy)."
    = { spawn_x | Number, spawn_y | Number, w | Number, h | Number },

  Objective
    | doc "A tracked objective for the ledger. `room = null` is serde's `None`."
    = {
      id | String,
      kind | ObjectiveKind,
      label | String,
      room | Nullable String,
    },

  ActionSpec
    | doc "One uplink action's economics."
    = { cost | NonNegative, cooldown | NonNegative, alert_gain | Number },

  Actions
    | doc "The uplink actions, keyed by `ActionKind`. `validate()` tolerates absent actions, so each key is optional; unknown keys are rejected (serde cannot parse them as `ActionKind`)."
    = {
      Camera | ActionSpec | optional,
      Door | ActionSpec | optional,
      Vacuum | ActionSpec | optional,
      Lights | ActionSpec | optional,
    },

  DifficultyPreset
    | doc "A full difficulty preset."
    = {
      label | String,
      arrival | OrderedPair,
      player_speed | Number,
      sprint | Number,
      billy_speed | Number,
      billy_sight | Number,
      support_limit | Number,
      bandwidth_regen | Number,
      badge_delay | Number,
      usb_timer | Number,
      camera_lock | Number,
      alert_gain | Number,
      score_mult | Number,
      rescue | Bool,
      trace_threshold | U32,
    },

  Difficulties
    | doc "All three presets, keyed by `DifficultyId` — every key is required (mirrors DifficultyMissing)."
    = {
      Story | DifficultyPreset,
      Standard | DifficultyPreset,
      Operator | DifficultyPreset,
    },

  GeometryTuning = {
    floor | Number,
    room_offset | Number,
    player_w | Number,
    player_h | Number,
    billy_w | Number,
    billy_h | Number,
    collision_radius | Number,
    world_width | Number,
  },

  MovementTuning = {
    gravity | Number,
    jump_vy | Number,
    accel_drive | Number,
    accel_brake | Number,
    crouch_speed | Number,
    stam_drain | Number,
    stam_regen | Number,
    stam_regen_crouch | Number,
    sprint_min_stam | Number,
    hide_max_vx | Number,
    crouch_cam_vx | Number,
    moving_vx | Number,
    clamp_lo | Number,
    clamp_hi | Number,
    land_vy | Number,
  },

  NoiseTuning = {
    jump | Number,
    land | Number,
    decay | Number,
    sprint_base | Number,
    sprint_k | Number,
    walk_k | Number,
    crouch | Number,
    move_min_vx | Number,
  },

  FsmTuning = {
    start_x | Number,
    crisis_x | Number,
    enter_speed | Number,
    shock_t | Number,
    assess_speed | Number,
    patrol_lo | Number,
    patrol_hi | Number,
    patrol_pivot | Number,
    invest_speed | Number,
    secure_speed | Number,
    grab_dist | Number,
    guard_speed | Number,
    guard_t_note | Number,
    guard_t_usb | Number,
    call_t | Number,
    pursue_speed | Number,
    pursue_trigger | Number,
    pursue_giveup_ago | Number,
    pursue_giveup_dist | Number,
    accel | Number,
    clamp_lo | Number,
    clamp_hi | Number,
    badge_open | Number,
    snack_reach | Number,
    alert_boost_div | Number,
    lights_boost_k | Number,
    vac_distract | Number,
    sight_back | Number,
    sight_crouch | Number,
    sight_office | Number,
    sight_alert | Number,
    sight_alert_hi | Number,
    invest_noise | Number,
    invest_dist | Number,
  },

  BeliefTuning = {
    threshold | Number,
    misdirect_threshold | Number,
    moving_vx | Number,
    pi_seen | Number,
    pi_sprint | Number,
    note_near | Number,
    usb_near | Number,
    note_urg_sprint | Number,
    note_urg_move | Number,
    note_urg_still | Number,
    usb_urg_sprint | Number,
    usb_urg_move | Number,
    usb_urg_still | Number,
    note_carry | Number,
    usb_carry | Number,
    prog_gate | Number,
    pi_decay | Number,
    note_decay | Number,
    usb_decay | Number,
    peel_interest | Number,
    secure_note_seen | Number,
    take_usb_seen | Number,
    take_usb_unseen | Number,
    camera_player_int | Number,
  },

  SupportTuning = {
    ping_hall | Number,
    ping_office | Number,
    ping_laundry | Number,
    ping_exit | Number,
    lowbw_x | Number,
    lowbw_pen | Number,
    alert_pen | Number,
    hidden | Number,
    flicker | Number,
    hop_pen | Number,
    clamp_min | Number,
    approach | Number,
    iso_gate | Number,
    iso_pressure_div | Number,
    fray_frac | Number,
    iso_decay | Number,
    iso_decay_hidden | Number,
    ping_dur | Number,
  },

  BandwidthTuning = {
    regen_a | Number,
    regen_b | Number,
    alert_decay_quiet | Number,
    alert_decay_crisis | Number,
    lockdown | Number,
  },

  ActionTiming = {
    lights_dur | Number,
    lights_dur_rm | Number,
    lights_stun | Number,
    route_duration | Number,
    throttle_cdbw | Number,
    throttle_fray | Number,
    throttle_badge | Number,
    call_alert | Number,
  },

  CameraTuning = {
    sweep_w | Number,
    sweep_a | Number,
    lockout | Number,
    alert | Number,
    decay_unseen | Number,
    decay_off | Number,
  },

  UsbTuning = {
    throw_vx | Number,
    throw_vy | Number,
    throw_off | Number,
    held_off | Number,
    grav | Number,
    drag | Number,
    floor | Number,
    bounce | Number,
    friction | Number,
    rest_vx | Number,
    rest_vy | Number,
    clamp_lo | Number,
    clamp_hi | Number,
    wipe_alert | Number,
    throw_alert | Number,
  },

  VacuumTuning = {
    lag_x | Number,
    ctrl_loss | Number,
    ctrl_min | Number,
    lag_warn | Number,
    ctrl_gain | Number,
    speed | Number,
    move_min | Number,
    target | Number,
    fall_off | Number,
  },

  InteractionTuning = {
    note_dist | Number,
    note_hold | Number,
    usb_dist | Number,
    chute_enter | Number,
    chute_search_off | Number,
    chute_search_dist | Number,
    chute_hold | Number,
    pickpocket_dist | Number,
    pickpocket_hold | Number,
    exit_prompt_x | Number,
    exit_x | Number,
  },

  RescueTuning = {
    support_min | Number,
    bw_min | Number,
    bw_cost | Number,
    stun | Number,
    grace | Number,
    displace | Number,
  },

  Tuning
    | doc "The full bundle of ported constants."
    = {
      geometry | GeometryTuning,
      movement | MovementTuning,
      noise | NoiseTuning,
      fsm | FsmTuning,
      belief | BeliefTuning,
      support | SupportTuning,
      bandwidth | BandwidthTuning,
      action | ActionTiming,
      camera | CameraTuning,
      usb | UsbTuning,
      vacuum | VacuumTuning,
      interaction | InteractionTuning,
      rescue | RescueTuning,
    },

  GradeBands
    | doc "Grade thresholds (raw score, pre score-multiplier)."
    = { s | Number, a | Number, b | Number, c | Number, fail_c | Number },

  Scoring
    | doc "The scoring/debrief weight table."
    = {
      base | Number,
      note | Number,
      note_lost | Number,
      note_none | Number,
      misdir | Number,
      misdir_leak | Number,
      noboss | Number,
      boss | Number,
      nocam | Number,
      cam_each | Number,
      iso_ok | Number,
      iso_snap | Number,
      iso_snap_frac | Number,
      chute | Number,
      usbtrace | Number,
      rescue | Number,
      time_base | Number,
      time_k | Number,
      alert_k | Number,
      fail_act | Number,
      fail_note | Number,
      fail_base | Number,
      fail_usb | Number,
      fail_alert_k | Number,
      grades | GradeBands,
    },

  SpawnRanges
    | doc "The (base, span) reset-RNG ranges — draw order is load-bearing."
    = {
      note_x | Pair,
      usb_x | Pair,
      stale_pulse | Pair,
      snack_x | Pair,
      door_delay | Pair,
      operator_door_penalty | Number,
    },

  Shape
    | doc "The field-level shape of a scenario definition (no cross-field checks)."
    = {
      format | String,
      scenario_id | String,
      name | String,
      floor_id | U32,
      world | World,
      rooms | Array Room,
      doors | Array Door,
      hide_spots | Array HideSpot,
      cameras | Array Camera,
      props | Props,
      player | Body,
      billy | Body,
      objectives | Array Objective,
      actions | Actions,
      difficulty | Difficulties,
      tuning | Tuning,
      scoring | Scoring,
      spawn | SpawnRanges,
    },

  ScenarioDefinition
    | doc "The complete scenario contract: the shape plus every cross-field check `ScenarioDefinition::validate` performs."
    = std.contract.Sequence [
      Shape,
      rooms_present,
      unique_room_ids,
      unique_door_ids,
      rooms_contiguous,
      doors_on_boundaries,
      hide_spot_rooms_exist,
      camera_rooms_exist,
      objective_rooms_exist,
      player_spawn_in_bounds,
      prop_spawns_in_bounds,
      extraction_objective_exists,
      cameras_fit_subnet,
      doors_fit_room_subnets,
    ],
}
