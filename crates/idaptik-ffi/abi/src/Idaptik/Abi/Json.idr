||| The JSON wire contract of `idap_ghost_lobby_tick_json` and
||| `idap_ghost_lobby_snapshot_json`: which results are arrays, which are
||| objects, and what a caller can tell without parsing.
|||
||| The tick contract is asymmetric on purpose: success is a JSON *array*
||| of events and recoverable failure is an `{"error":"..."}` *object*, so
||| shape alone classifies the result. The snapshot contract is not: both
||| success and failure are objects, so a caller must parse — the model
||| keeps that honest asymmetry instead of flattening it.
module Idaptik.Abi.Json

import Idaptik.Abi.Run

%default total

||| The two wire shapes a tick result can take.
public export
data TickShape = TickEvents | TickError

||| The character each shape's rendered form opens with.
public export
shapeChar : TickShape -> Char
shapeChar TickEvents = '['
shapeChar TickError  = '{'

||| A tick result, indexed by the wire shape it renders to.
public export
data TickResult : TickShape -> Type where
  EventsResult : (events : List String) -> TickResult TickEvents
  ErrorResult  : (message : String) -> TickResult TickError

joinBy : String -> List String -> String
joinBy _ []        = ""
joinBy _ [x]       = x
joinBy sep (x::xs) = x ++ sep ++ joinBy sep xs

||| Head of a list, for the opening-character contract below.
export
headChar : List Char -> Maybe Char
headChar []       = Nothing
headChar (c :: _) = Just c

||| The rendered form, as a character list, so the opening-character
||| contract is checkable rather than asserted.
export
renderChars : TickResult shape -> List Char
renderChars (EventsResult events) =
  '[' :: (unpack (joinBy "," events) ++ (']' :: []))
renderChars (ErrorResult message) =
  '{' :: (unpack ("\"error\":") ++ (unpack message ++ ('}' :: [])))

||| The rendered JSON wire form of a tick result.
public export
renderTick : TickResult shape -> String
renderTick = pack . renderChars

||| CHECKED: the first character of a rendered tick result identifies its
||| shape. A success array can never be confused with an error object —
||| the distinction the header promises callers, as an equation.
public export
renderTickIdentifiesShape : (r : TickResult shape) ->
                            headChar (renderChars r) = Just (shapeChar shape)
renderTickIdentifiesShape (EventsResult _) = Refl
renderTickIdentifiesShape (ErrorResult _)  = Refl

||| A snapshot result: a restorable snapshot object on success, an error
||| object on recoverable failure. Both open with '{'; shape alone cannot
||| tell them apart — matching the C contract, unlike the tick pair.
public export
data SnapshotResult : Type where
  SnapshotOk   : RuntimeSnapshot -> SnapshotResult
  SnapshotFail : (message : String) -> SnapshotResult

||| An owned wire string the library handed back, tagged with the character
||| its rendered form opens with. The tag is what a caller can observe
||| without parsing; `idap_string_free` is the only way to consume it.
public export
data OwnedWire : Type where
  MkOwnedWire : (opener : Char) -> (rendered : String) -> OwnedWire

public export
shapeOf : OwnedWire -> Char
shapeOf (MkOwnedWire opener _) = opener

||| Wrap a tick result as the owned string `idap_ghost_lobby_tick_json`
||| returns.
public export
ownTick : TickResult shape -> OwnedWire
ownTick r = MkOwnedWire (shapeChar _) (renderTick r)

||| CHECKED: the owned wire a tick hands back is tagged with its shape's
||| opening character.
public export
ownTickTaggedWithOpener : (r : TickResult shape) ->
                          shapeOf (ownTick r) = shapeChar shape
ownTickTaggedWithOpener _ = Refl

||| Wrap a snapshot result. Both cases open with '{' — see `SnapshotResult`.
public export
renderSnapshot : SnapshotResult -> String
renderSnapshot (SnapshotOk snap) =
  "{\"format\":\"" ++ snap.format ++ "\",\"tick\":" ++ show (tick (state snap)) ++ "}"
renderSnapshot (SnapshotFail message) =
  "{\"error\":\"" ++ message ++ "\"}"

public export
ownSnapshot : SnapshotResult -> OwnedWire
ownSnapshot r = MkOwnedWire '{' (renderSnapshot r)
