||| The deterministic Ghost Lobby run as a pure function, plus the
||| snapshot format invariants — the half of issue #103 that lives on the
||| Rust side of the boundary, modelled so the ABI layer can promise it.
|||
||| Vocabulary note: `Command`/`Event` here are the structural core of
||| `idaptik-core`'s enums (the startup events, the held-button rule, phase
||| changes), not the full sets. Bridging the model to the full serde wire
||| is the conformance-test work issue #103 tracks.
module Idaptik.Abi.Run

import Decidable.Equality

import Idaptik.Abi.CABI

%default total

||| Movement buttons, as in `idaptik_core::scenario::command::Button`.
public export
data Button = Left | Right | Up | Down

public export
Eq Button where
  (==) Left Left   = True
  (==) Right Right = True
  (==) Up Up       = True
  (==) Down Down   = True
  (==) _ _         = False

||| The per-tick command stream the header's `commands_json` folds into one
||| `TickInput`: held buttons persist across ticks; the rest are edges.
public export
data Command = SetButton Button Bool | Jump | Interact | ThrowUsb

||| The structural core of `idaptik_core::scenario::event::Event`.
public export
data Event
  = RunStarted Bits32 Difficulty Bool
  | SeedAnnounced Bits32
  | PhaseChanged String String
  | CrisisBegan String

||| The held-button set: `SetButton` presses stay down across later ticks
||| until released, exactly as the TUI's `InputState::sample` feeds the sim.
public export
HeldSet : Type
HeldSet = Button -> Bool

emptyHeld : HeldSet
emptyHeld = const False

hold : Button -> Bool -> HeldSet -> HeldSet
hold b down held = \b' => if b' == b then down else held b'

||| A run's state: the seed and difficulty it began with, the current
||| 60 Hz frame counter, the persistent held set, and the startup events
||| queued for the first tick (`RunStarted`, `SeedAnnounced`).
public export
record RunState where
  constructor MkRunState
  seed : Bits32
  difficulty : Difficulty
  tick : Nat
  held : HeldSet
  queued : List Event

||| `idap_ghost_lobby_new`'s successful path: a fresh run with the startup
||| events queued ahead of the first tick's own events.
public export
newRun : Bits32 -> Difficulty -> RunState
newRun seed difficulty =
  MkRunState seed difficulty 0 emptyHeld
    [ RunStarted seed difficulty False
    , SeedAnnounced seed
    ]

||| One deterministic 60 Hz frame: fold this tick's `SetButton`s into the
||| persistent held set, advance the frame counter, and emit this tick's
||| events with any queued startup events in front.
public export
step : RunState -> List Command -> (RunState, List Event)
step s cmds =
  ( MkRunState (seed s) (difficulty s) (S (tick s))
      (foldl applyCommand (held s) cmds) []
  , queued s ++ eventsFor (S (tick s)) cmds
  )
  where
    applyCommand : HeldSet -> Command -> HeldSet
    applyCommand h (SetButton b down) = hold b down h
    applyCommand h _                  = h

    ||| The structural events a tick itself emits. The core emits the run's
    ||| real event stream; the model keeps the phase machinery: the first
    ||| frame ends the briefing phase.
    eventsFor : Nat -> List Command -> List Event
    eventsFor (S Z) _ = [PhaseChanged "briefing" "infiltration"]
    eventsFor _     _ = []

||| A whole run is a pure function of its start state and command stream.
||| ADR-0004's determinism is not asserted here; it is the shape of `run`.
public export
run : RunState -> List (List Command) -> (RunState, List Event)
run s []        = (s, [])
run s (cs :: rest) =
  let (s1, e1) = step s cs
      (s2, e2) = run s1 rest
   in (s2, e1 ++ e2)

||| CHECKED: one tick advances the frame counter by exactly one.
public export
stepAdvancesTick : (s : RunState) -> (cs : List Command) ->
                   tick (fst (step s cs)) = S (tick s)
stepAdvancesTick _ _ = Refl

||| CHECKED: the header's rule that held buttons persist across ticks — a
||| `SetButton` press on one tick is still down on a later idle tick.
public export
buttonPersistsAcrossTicks : (b : Button) -> (down : Bool) -> (s : RunState) ->
  held (fst (step (fst (step s [SetButton b down])) [])) b = down
buttonPersistsAcrossTicks Left  down s = Refl
buttonPersistsAcrossTicks Right down s = Refl
buttonPersistsAcrossTicks Up    down s = Refl
buttonPersistsAcrossTicks Down  down s = Refl

||| The snapshot format tag, `SNAPSHOT_FORMAT` in
||| `idaptik_core::scenario::snapshot`. The gate script diffs `formatTag`
||| against the Rust constant so the two cannot drift apart silently.
public export
data SnapshotFormat = RuntimeV3

public export
formatTag : SnapshotFormat -> String
formatTag RuntimeV3 = "idaptik-ghost-lobby-runtime-v3"

||| A full, restorable snapshot of the run at the current tick.
public export
record RuntimeSnapshot where
  constructor MkRuntimeSnapshot
  format : String
  state  : RunState

||| Serialise the current run state with the current format tag.
public export
snapshotOf : RunState -> RuntimeSnapshot
snapshotOf s = MkRuntimeSnapshot (formatTag RuntimeV3) s

||| Restore accepts only the current format tag; any other string is
||| rejected, exactly as `RuntimeSnapshot::from_json` checks `SNAPSHOT_FORMAT`.
public export
restore : RuntimeSnapshot -> Maybe RunState
restore snap = case decEq snap.format (formatTag RuntimeV3) of
  Yes _   => Just snap.state
  No  _   => Nothing

||| CHECKED: a snapshot this model serialises restores to exactly the run
||| state it was taken from.
public export
snapshotRoundTrip : (s : RunState) -> restore (snapshotOf s) = Just s
snapshotRoundTrip s with (decEq (formatTag RuntimeV3) (formatTag RuntimeV3))
  snapshotRoundTrip s | Yes _  = Refl
  snapshotRoundTrip s | No contra = absurd (contra Refl)
