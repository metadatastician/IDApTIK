||| The C call protocol as a type: which sequences of `idap_*` calls a
||| client may make, and what must be true when it stops.
|||
||| `Handles` proves that a *single* free is well-typed. That is not the
||| obligation the header states. The header's `# Safety` sections promise
||| something about whole call sequences: a handle is freed exactly once,
||| every string handed out is handed back, and nothing touches a handle
||| after it is freed. Those are statements about traces, so this module
||| makes the trace the object of study.
|||
||| One index carries both dimensions the header couples:
|||
|||   * `HandleState` — the lifetime of the one ghost-lobby handle;
|||   * `Nat`         — how many owned wire strings the library has handed
|||                     out that the client has not yet freed.
|||
||| Freeing a string is deliberately polymorphic in the handle state: C
||| permits `idap_string_free` after `idap_ghost_lobby_free`, because the
||| string is a separate allocation. Modelling that faithfully is what makes
||| the exactly-once theorem non-trivial — `Session Freed _ Freed _` is
||| inhabited, so the induction must count `Free` constructors specifically
||| rather than appeal to the handle index alone.
module Idaptik.Abi.Protocol

import Idaptik.Abi.CABI
import Idaptik.Abi.Handles
import Idaptik.Abi.Json
import Idaptik.Abi.Exports
import Data.List
import Data.List.Elem
import Idaptik.Abi.Run
import Data.Nat

%default total

||| One C entry-point call, indexed by the state it requires and the state
||| it leaves. The indices *are* the precondition: there is no way to write
||| a call that the ABI forbids.
public export
data Op : HandleState -> Nat -> HandleState -> Nat -> Type where
  ||| `idap_ghost_lobby_tick_json` — needs a live handle, hands back an
  ||| owned string the client now owes a free for.
  Tick     : (cmds : List Command) -> Op Live n Live (S n)
  ||| `idap_ghost_lobby_snapshot_json` — likewise.
  Snapshot : Op Live n Live (S n)
  ||| `idap_string_free` on a string the library handed out. Legal whether
  ||| or not the handle is still live.
  FreeStr  : Op h (S n) h n
  ||| `idap_string_free(NULL)` — the header's documented no-op.
  FreeNull : Op h n h n
  ||| `idap_ghost_lobby_free` — consumes the live handle.
  Free     : Op Live n Freed n

||| A sequence of calls. Well-typedness of the sequence is the ABI
||| discipline: a use-after-free simply has no typing derivation.
public export
data Session : HandleState -> Nat -> HandleState -> Nat -> Type where
  Done : Session h n h n
  (::) : Op h1 n1 h2 n2 -> Session h2 n2 h3 n3 -> Session h1 n1 h3 n3

||| A complete client: starts with a live handle owing nothing, ends with
||| the handle freed and no string outstanding.
public export
Complete : Type
Complete = Session Live 0 Freed 0

-- --------------------------------------------------------------------------
-- Counting. These are the quantities the header's prose talks about.
-- --------------------------------------------------------------------------

||| How many times the session calls `idap_ghost_lobby_free`.
public export
freeCount : Session h1 n1 h2 n2 -> Nat
freeCount Done          = 0
freeCount (Free :: rest) = S (freeCount rest)
freeCount (_    :: rest) = freeCount rest

||| How many owned strings the library hands out.
public export
emitCount : Session h1 n1 h2 n2 -> Nat
emitCount Done              = 0
emitCount (Tick _   :: rest) = S (emitCount rest)
emitCount (Snapshot :: rest) = S (emitCount rest)
emitCount (_        :: rest) = emitCount rest

||| How many of them the client hands back. `FreeNull` is not counted: it
||| frees nothing, which is exactly why it may appear anywhere.
public export
freeStrCount : Session h1 n1 h2 n2 -> Nat
freeStrCount Done             = 0
freeStrCount (FreeStr :: rest) = S (freeStrCount rest)
freeStrCount (_       :: rest) = freeStrCount rest

-- --------------------------------------------------------------------------
-- Obligation 1: ownership and lifetime.
-- --------------------------------------------------------------------------

||| CHECKED: nothing resurrects a freed handle. Every operation that could
||| leave a handle live demands a live one to begin with.
public export
noResurrection : Op Freed n Live m -> Void
noResurrection (Tick _) impossible
noResurrection Snapshot impossible
noResurrection FreeStr  impossible
noResurrection FreeNull impossible
noResurrection Free     impossible

||| CHECKED: once freed, a session never frees again. This is the half of
||| exactly-once that rules out a double free, and it is where the absence
||| of any `Op Freed _ Live _` does its work: the only calls still available
||| are the two string frees.
public export
noFreeAfterFreed : (s : Session Freed n1 h2 n2) -> freeCount s = 0
noFreeAfterFreed Done              = Refl
noFreeAfterFreed (FreeStr  :: rest) = noFreeAfterFreed rest
noFreeAfterFreed (FreeNull :: rest) = noFreeAfterFreed rest

||| CHECKED: a session that starts live and ends freed calls
||| `idap_ghost_lobby_free` exactly once — never zero times (a leak), never
||| twice (a double free).
public export
liveToFreedFreesOnce : (s : Session Live n1 Freed n2) -> freeCount s = 1
liveToFreedFreesOnce (Tick _   :: rest) = liveToFreedFreesOnce rest
liveToFreedFreesOnce (Snapshot :: rest) = liveToFreedFreesOnce rest
liveToFreedFreesOnce (FreeStr  :: rest) = liveToFreedFreesOnce rest
liveToFreedFreesOnce (FreeNull :: rest) = liveToFreedFreesOnce rest
liveToFreedFreesOnce (Free     :: rest) = cong S (noFreeAfterFreed rest)

||| CHECKED: the header's ownership promise, for a complete client.
public export
completeFreesExactlyOnce : (s : Complete) -> freeCount s = 1
completeFreesExactlyOnce = liveToFreedFreesOnce

-- --------------------------------------------------------------------------
-- Obligation 2: every string handed out is handed back.
-- --------------------------------------------------------------------------

||| CHECKED: across any session, the strings owed at the start plus those
||| emitted equal those freed plus those still owed at the end. Nothing
||| leaks and nothing is freed twice, at every prefix — not merely at the
||| end.
public export
sessionBalance : {n1 : Nat} -> (s : Session h1 n1 h2 n2) ->
                 n1 + emitCount s = freeStrCount s + n2
sessionBalance Done = plusZeroRightNeutral n1
sessionBalance (Tick _ :: rest) =
  rewrite sym (plusSuccRightSucc n1 (emitCount rest)) in sessionBalance rest
sessionBalance (Snapshot :: rest) =
  rewrite sym (plusSuccRightSucc n1 (emitCount rest)) in sessionBalance rest
sessionBalance (FreeStr  :: rest) = cong S (sessionBalance rest)
sessionBalance (FreeNull :: rest) = sessionBalance rest
sessionBalance (Free     :: rest) = sessionBalance rest

||| CHECKED: a complete client frees exactly as many strings as the library
||| handed it. `idap_string_free` is called once per emitted string.
public export
completeBalancesStrings : (s : Complete) -> emitCount s = freeStrCount s
completeBalancesStrings s =
  rewrite sym (plusZeroRightNeutral (freeStrCount s)) in sessionBalance s

-- --------------------------------------------------------------------------
-- Witnesses. A theorem about an uninhabited type is true and worthless, so
-- the discipline is exhibited, not merely constrained.
-- --------------------------------------------------------------------------

||| A realistic client: tick, read the string, free it; snapshot, free it;
||| a stray `idap_string_free(NULL)`; then free the handle.
public export
typicalSession : Complete
typicalSession =
  Tick [Jump] :: FreeStr :: Snapshot :: FreeStr :: FreeNull :: Free :: Done

||| The case that makes the exactly-once theorem non-trivial: C permits
||| freeing the string *after* the handle, so the session visits `Freed`
||| with work still to do. `Session Freed _ Freed _` is inhabited.
public export
lateStringFree : Complete
lateStringFree = Tick [Jump] :: Free :: FreeStr :: Done

||| POSITIVE CONTROLS: the theorems above say something about these.
public export
typicalFreesOnce : freeCount Protocol.typicalSession = 1
typicalFreesOnce = Refl

public export
lateFreesOnce : freeCount Protocol.lateStringFree = 1
lateFreesOnce = Refl

public export
typicalBalances : emitCount Protocol.typicalSession = 2
typicalBalances = Refl

public export
typicalFreesTwoStrings : freeStrCount Protocol.typicalSession = 2
typicalFreesTwoStrings = Refl

-- --------------------------------------------------------------------------
-- The protocol denotes the real ABI, not a toy.
--
-- Everything above is a fact about an index discipline. These two
-- definitions are what make it a fact about `idaptik-ffi`: every operation
-- names a function the C header actually exports, and interpreting a
-- session runs the model's own `step`/`free` on a linear handle.
-- --------------------------------------------------------------------------

||| The C entry point each operation denotes.
public export
entryPoint : Op h1 n1 h2 n2 -> String
entryPoint (Tick _) = "idap_ghost_lobby_tick_json"
entryPoint Snapshot = "idap_ghost_lobby_snapshot_json"
entryPoint FreeStr  = "idap_string_free"
entryPoint FreeNull = "idap_string_free"
entryPoint Free     = "idap_ghost_lobby_free"

||| CHECKED: every operation the protocol permits denotes a function the
||| header exports. The protocol cannot drift from the C surface.
|||
||| This is a constructive membership proof (`Data.List.Elem`), not a
||| `Bool = True`: `prim__eq_String` does not reduce during unification, so a
||| decidable-equality phrasing would not close by `Refl`. The position of
||| each name is part of the proof, so reordering `exportedFunctions` breaks
||| the build rather than silently weakening it.
|||
||| MEASURED trap: the name must be written **qualified**. A bare
||| `exportedFunctions` does not delta-reduce during unification even though
||| it is `public export` — the identical proof term fails with
||| "Mismatch between: ?y :: ... and exportedFunctions" and succeeds verbatim
||| as `Exports.exportedFunctions`.
|||
||| `exportedFunctions` is the list `scripts/abi_model_check.sh` diffs against
||| `include/idaptik.h`, so this closes the loop: header to model by the gate,
||| model to protocol by this proof.
public export
entryPointIsExported : (op : Op h1 n1 h2 n2) -> Elem (entryPoint op) Exports.exportedFunctions
entryPointIsExported (Tick _) = There (There (There (There Here)))
entryPointIsExported Snapshot = There (There (There (There (There Here))))
entryPointIsExported FreeStr  = There (There (There (There (There (There (There Here))))))
entryPointIsExported FreeNull = There (There (There (There (There (There (There Here))))))
entryPointIsExported Free     = There (There (There (There (There (There Here)))))

||| Run one operation on a linear handle. The handle is consumed and a
||| handle in the operation's target state is returned, so the index
||| discipline above and the linear discipline in `Handles` are the same
||| discipline.
public export
applyOp : (1 h : Handle GhostLobbyKind s1) -> Op s1 n1 s2 n2 ->
          Handle GhostLobbyKind s2
applyOp (LiveHandle st) (Tick cmds) = LiveHandle (fst (step st cmds))
applyOp (LiveHandle st) Snapshot    = LiveHandle st
applyOp h               FreeStr     = h
applyOp h               FreeNull    = h
applyOp h               Free        = free h

||| Interpret a whole session against a linear handle.
public export
interpret : (1 h : Handle GhostLobbyKind s1) -> Session s1 n1 s2 n2 ->
            Handle GhostLobbyKind s2
interpret h Done          = h
interpret h (op :: rest)  = interpret (applyOp h op) rest
