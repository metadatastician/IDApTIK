||| The exported surface of `crates/idaptik-ffi/include/idaptik.h`, one
||| model declaration per header function, in header order.
|||
||| `scripts/abi_model_check.sh` diffs `exportedFunctions` against the
||| header and scans the package for escape hatches (`postulate`,
||| `believe_me`, `assert_total`, partial definitions), so the model stays
||| total, honest, and in lockstep with the C surface.
|||
||| Two deliberate idealisations, both noted where they occur:
||| a C const borrow does not transfer ownership, which linear types cannot
||| express directly — the model threads the handle through the observation
||| and hands it back; and the simplified commands wire is parsed here only
||| far enough to exercise the fold — the serde JSON bridge is the
||| conformance-test work issue #103 tracks.
module Idaptik.Abi.Exports

import Idaptik.Abi.CABI
import Idaptik.Abi.Handles
import Idaptik.Abi.Json
import Idaptik.Abi.Run

%default total

||| Every function `idaptik.h` exports, in header order. The gate script
||| diffs this against the header so the model cannot drift from the C
||| surface in either direction.
public export
exportedFunctions : List String
exportedFunctions =
  [ "idap_demo_network"
  , "idap_network_free"
  , "idap_network_device_count"
  , "idap_ghost_lobby_new"
  , "idap_ghost_lobby_tick_json"
  , "idap_ghost_lobby_snapshot_json"
  , "idap_ghost_lobby_free"
  , "idap_string_free"
  ]

||| Header: `struct NetworkHandle *idap_demo_network(void);`
||| The caller owns the result and must release it with
||| `idap_network_free`. The model hands the handle over linearly: exactly
||| one free is possible.
public export
idap_demo_network : () -> CNullable (Handle NetworkKind Live)
idap_demo_network () = CSome (LiveHandle (MkNetworkModel 4))

||| Header: `void idap_network_free(struct NetworkHandle *ptr);`
||| Passing null is a no-op; a live handle transitions to Freed.
public export
idap_network_free : (1 p : CNullable (Handle NetworkKind Live)) ->
                    CNullable (Handle NetworkKind Freed)
idap_network_free = freeNullable

||| Header: `uintptr_t idap_network_device_count(const struct NetworkHandle *ptr);`
||| Returns 0 for a null pointer. A C const borrow does not transfer
||| ownership, which the model cannot say directly; it threads the handle
||| linearly through the observation and hands it back alongside the count.
public export
idap_network_device_count : (1 p : CNullable (Handle NetworkKind Live)) ->
                            LPair (CNullable (Handle NetworkKind Live)) Nat
idap_network_device_count CNull                = CNull # 0
idap_network_device_count (CSome (LiveHandle net)) =
  CSome (LiveHandle net) # deviceCount net

||| Header: `struct GhostLobbyHandle *idap_ghost_lobby_new(uint32_t seed, const char *difficulty);`
||| Returns null — never panics — for a null or invalid difficulty; on
||| success the startup events (`RunStarted`, `SeedAnnounced`) are queued
||| inside the sim for the first tick to return.
public export
idap_ghost_lobby_new : (seed : Bits32) -> (difficulty : CNullable CString) ->
                       CNullable (Handle GhostLobbyKind Live)
idap_ghost_lobby_new seed (CSome diff) =
  case parseDifficulty (text diff) of
    Just d  => CSome (LiveHandle (newRun seed d))
    Nothing => CNull
idap_ghost_lobby_new _ CNull = CNull

splitOn : Char -> String -> List String
splitOn sep s = map pack (splitChars sep (unpack s))
  where
    splitChars : Char -> List Char -> List (List Char)
    splitChars sep []        = []
    splitChars sep (c :: cs) =
      if c == sep then [] :: splitChars sep cs
      else case splitChars sep cs of
        []         => [(c :: [])]
        (w :: ws)  => (c :: w) :: ws

||| The simplified wire the model's tick parser accepts: `;`-separated
||| commands, each `Jump` | `Interact` | `ThrowUsb` |
||| `SetButton:<Left|Right|Up|Down>:<down|up>`. The real wire is the serde
||| JSON the Elixir session layer and the TUI share; bridging the two is
||| the conformance-test work issue #103 tracks.
parseCommands : String -> Maybe (List Command)
parseCommands s =
  if trim s == "" then Just [] else parseAll (splitOn ';' s)
  where
    trim : String -> String
    trim = pack . dropSp . reverse . dropSp . reverse . unpack
      where
        dropSp : List Char -> List Char
        dropSp []        = []
        dropSp (c :: cs) = if c == ' ' then dropSp cs else c :: cs

    parseButtonPart : String -> Maybe Button
    parseButtonPart "Left"  = Just Idaptik.Abi.Run.Left
    parseButtonPart "Right" = Just Idaptik.Abi.Run.Right
    parseButtonPart "Up"    = Just Idaptik.Abi.Run.Up
    parseButtonPart "Down"  = Just Idaptik.Abi.Run.Down
    parseButtonPart _       = Nothing

    parseDown : String -> Maybe Bool
    parseDown "down" = Just True
    parseDown "up"   = Just False
    parseDown _      = Nothing

    parseCommand : String -> Maybe Command
    parseCommand "Jump"     = Just Jump
    parseCommand "Interact" = Just Interact
    parseCommand "ThrowUsb" = Just ThrowUsb
    parseCommand c = case splitOn ':' c of
      ["SetButton", button, down] => case parseButtonPart button of
        Nothing => Nothing
        Just b  => case parseDown down of
          Nothing => Nothing
          Just d  => Just (SetButton b d)
      _ => Nothing

    parseAll : List String -> Maybe (List Command)
    parseAll []        = Just []
    parseAll (x :: xs) = case parseCommand x of
      Nothing => Nothing
      Just c  => case parseAll xs of
        Nothing => Nothing
        Just cs => Just (c :: cs)

||| The structural tag of an event, as the model renders it into the
||| events array.
eventTag : Event -> String
eventTag (RunStarted _ _ _)   = "RunStarted"
eventTag (SeedAnnounced _)    = "SeedAnnounced"
eventTag (PhaseChanged _ _)   = "PhaseChanged"
eventTag (CrisisBegan _)      = "CrisisBegan"

||| Header: `char *idap_ghost_lobby_tick_json(struct GhostLobbyHandle *ptr, const char *commands_json);`
||| Advance the run by exactly one fixed 60 Hz frame. Out contract: an
||| events array on success, an `{"error":"..."}` object on a recoverable
||| failure, null only for a null handle or allocation failure.
public export
idap_ghost_lobby_tick_json : (1 p : CNullable (Handle GhostLobbyKind Live)) ->
                             (commands : CNullable CString) ->
                             LPair (CNullable (Handle GhostLobbyKind Live))
                                   (CNullable OwnedWire)
idap_ghost_lobby_tick_json CNull _ = CNull # CNull
idap_ghost_lobby_tick_json (CSome (LiveHandle st)) commands =
  case commands of
    CNull =>
      (CSome (LiveHandle st)) #
        CSome (ownTick (ErrorResult "commands_json is null"))
    CSome cs =>
      case parseCommands (text cs) of
        Nothing =>
          (CSome (LiveHandle st)) #
            CSome (ownTick (ErrorResult
              "commands_json is not a JSON array of commands"))
        Just cmds =>
          let (st', events) = step st cmds
           in (CSome (LiveHandle st')) #
                CSome (ownTick (EventsResult (map eventTag events)))

||| Header: `char *idap_ghost_lobby_snapshot_json(const struct GhostLobbyHandle *ptr);`
||| A full restorable `RuntimeSnapshot` of the run at the current tick.
||| Success and failure are both JSON objects here — a caller must parse to
||| tell them apart, unlike the tick pair (see `Idaptik.Abi.Json`).
public export
idap_ghost_lobby_snapshot_json : (1 p : CNullable (Handle GhostLobbyKind Live)) ->
                                 LPair (CNullable (Handle GhostLobbyKind Live))
                                       (CNullable OwnedWire)
idap_ghost_lobby_snapshot_json CNull = CNull # CNull
idap_ghost_lobby_snapshot_json (CSome (LiveHandle st)) =
  (CSome (LiveHandle st)) #
    CSome (ownSnapshot (SnapshotOk (snapshotOf st)))

||| Header: `void idap_ghost_lobby_free(struct GhostLobbyHandle *ptr);`
||| Passing null is a no-op; a live handle transitions to Freed.
public export
idap_ghost_lobby_free : (1 p : CNullable (Handle GhostLobbyKind Live)) ->
                        CNullable (Handle GhostLobbyKind Freed)
idap_ghost_lobby_free = freeNullable

||| Header: `void idap_string_free(char *s);`
||| Passing null is a no-op; an owned wire string is consumed exactly once
||| (see `Idaptik.Abi.CABI.stringFree` for the `Owned` form).
public export
idap_string_free : (1 w : CNullable OwnedWire) -> ()
idap_string_free CNull             = ()
idap_string_free (CSome (MkOwnedWire _ _)) = ()

||| A complete well-typed client session through the modelled ABI: start a
||| run, tick it, snapshot it, free the run, and free every owned string.
||| Typechecking is the proof that the protocol has no dead ends and that a
||| linear client releases each handle and each string exactly once.
public export
demoSession : Bits32 -> CString -> CString -> String
demoSession seed difficulty commands =
  case idap_ghost_lobby_new seed (CSome difficulty) of
    CNull => "new: null (invalid difficulty)"
    CSome h0 =>
      case idap_ghost_lobby_tick_json (CSome h0) (CSome commands) of
        (h1 # tickWire) =>
          case consumeWire tickWire of
            () =>
              case idap_ghost_lobby_snapshot_json h1 of
                (h2 # snapWire) =>
                  case consumeWire snapWire of
                    () =>
                      case idap_ghost_lobby_free h2 of
                        freed => case discardFreed freed of
                          () => "session: tick + snapshot + free complete"
  where
    consumeWire : (1 w : CNullable OwnedWire) -> ()
    consumeWire CNull                 = ()
    consumeWire w@(CSome _)           = idap_string_free w
