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
import Data.Nat

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

-- --------------------------------------------------------------------------
-- Obligation 4: deterministic tick, and snapshot invariants.
--
-- A word on what is NOT proved here. "Two runs from the same inputs give
-- the same output" is `Refl` in this model: `run` is a pure function, so
-- the equation holds by the definition of a function and states nothing
-- about IDApTIK. Shipping it would be a lemma that cannot fail. The
-- substance of ADR-0004's determinism is that the frame clock is driven by
-- the frame count ALONE — no command, seed or difficulty can skip,
-- duplicate or stall a frame — and that is `runAdvancesByFrameCount`,
-- which is a genuine induction with a genuine mutant.
-- --------------------------------------------------------------------------

||| The state half of a run. `run` returns state and events together; the
||| frame-clock theorems concern only the state, and separating them lets
||| the induction reduce.
public export
runState : RunState -> List (List Command) -> RunState
runState s []          = s
runState s (cs :: rest) = runState (fst (step s cs)) rest

||| CHECKED: `runState` really is the state `run` computes. Without this
||| the frame-clock theorems below would be about a function the ABI never
||| calls — true, and about nothing.
export
runFstIsRunState : (s : RunState) -> (css : List (List Command)) ->
                   fst (run s css) = runState s css
runFstIsRunState s []           = Refl
runFstIsRunState s (cs :: rest) with (run (fst (step s cs)) rest) proof eq
  runFstIsRunState s (cs :: rest) | (s2, e2) =
    trans (cong fst (sym eq)) (runFstIsRunState (fst (step s cs)) rest)

||| CHECKED: the frame counter after a run is the starting counter plus the
||| number of frames — whatever the commands were. No command can skip,
||| duplicate or stall a frame.
public export
runAdvancesByFrameCount : (s : RunState) -> (css : List (List Command)) ->
                          tick (runState s css) = tick s + length css
runAdvancesByFrameCount s []          = sym (plusZeroRightNeutral (tick s))
runAdvancesByFrameCount s (cs :: rest) =
  trans (runAdvancesByFrameCount (fst (step s cs)) rest)
        (plusSuccRightSucc (tick s) (length rest))

||| CHECKED: the frame clock is a function of the frame count alone. Two
||| runs that agree on the starting tick and the NUMBER of frames agree on
||| the final tick, however their seeds, difficulties and commands differ.
public export
frameClockIgnoresInputs : (s1, s2 : RunState) ->
                          (css1, css2 : List (List Command)) ->
                          tick s1 = tick s2 ->
                          length css1 = length css2 ->
                          tick (runState s1 css1) = tick (runState s2 css2)
frameClockIgnoresInputs s1 s2 css1 css2 sameTick sameLen =
  trans (runAdvancesByFrameCount s1 css1)
        (trans (cong2 plus sameTick sameLen)
               (sym (runAdvancesByFrameCount s2 css2)))

||| CHECKED: `restore` rejects any snapshot that is not this format. The
||| round-trip theorem says a good snapshot survives; this says a foreign
||| one is refused rather than silently reinterpreted — the half that makes
||| the format tag load-bearing.
public export
restoreRejectsForeignFormat : (tag : String) -> (s : RunState) ->
                              Not (tag = formatTag RuntimeV3) ->
                              restore (MkRuntimeSnapshot tag s) = Nothing
restoreRejectsForeignFormat tag s contra with (decEq tag (formatTag RuntimeV3))
  restoreRejectsForeignFormat tag s contra | Yes prf = absurd (contra prf)
  restoreRejectsForeignFormat tag s contra | No  _   = Refl

||| POSITIVE CONTROL: the rejection theorem is not vacuous — a wrong tag
||| really is refused.
public export
restoreRejectsV2 : restore (MkRuntimeSnapshot "idaptik-ghost-lobby-runtime-v2"
                                              (newRun 7 Standard)) = Nothing
restoreRejectsV2 = Refl

||| CHECKED: the frame-clock theorem, restated about `run` — the function
||| `idap_ghost_lobby_tick_json` actually drives.
public export
runAdvancesByFrameCountOnRun : (s : RunState) -> (css : List (List Command)) ->
                               tick (fst (run s css)) = tick s + length css
runAdvancesByFrameCountOnRun s css =
  trans (cong tick (runFstIsRunState s css)) (runAdvancesByFrameCount s css)
