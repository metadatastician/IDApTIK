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
import Data.Nat
import Data.List

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


-- --------------------------------------------------------------------------
-- Obligation 3: wire shape, and no interior NUL.
--
-- `renderTickIdentifiesShape` above proves what a caller learns from the
-- FIRST byte. The header promises more: the value is well-formed JSON, and
-- it is a C string, so it must contain no interior NUL. Both are proved
-- here for the forms the API actually produces.
--
-- Note on `show`: `show : Nat -> String` goes through a primitive cast and
-- does not reduce during elaboration, so nothing about its output is
-- provable. The digits are therefore produced by `natChars`, which is
-- ordinary structural recursion and whose output is digits by
-- construction. Fuel, not `assert_total`, is what makes it total.
-- --------------------------------------------------------------------------

||| The character each shape's rendered form closes with.
public export
closeChar : TickShape -> Char
closeChar TickEvents = ']'
closeChar TickError  = '}'

||| Last element, for the closing-character contract.
public export
lastChar : List Char -> Maybe Char
lastChar []              = Nothing
lastChar [c]             = Just c
lastChar (_ :: c :: cs)  = lastChar (c :: cs)

||| The last character of `xs ++ [c]` is `c`, whatever `xs` is.
export
lastSnoc : (xs : List Char) -> (c : Char) -> lastChar (xs ++ [c]) = Just c
lastSnoc []             c = Refl
lastSnoc [_]            c = Refl
lastSnoc (_ :: y :: xs) c = lastSnoc (y :: xs) c

||| The same, one concatenation deeper — proved by induction rather than
||| by rewriting, so no associativity lemma is needed.
export
lastAppSnoc : (xs, ys : List Char) -> (c : Char) ->
              lastChar (xs ++ (ys ++ [c])) = Just c
lastAppSnoc []             ys        c = lastSnoc ys c
lastAppSnoc [_]            []        c = Refl
lastAppSnoc [_]            (y :: ys) c = lastSnoc (y :: ys) c
lastAppSnoc (_ :: y :: xs) ys        c = lastAppSnoc (y :: xs) ys c

||| CHECKED: a rendered tick result closes with the bracket its shape opens
||| with. Together with `renderTickIdentifiesShape`, the value is delimited:
||| an events array is `[...]` and an error is `{...}`.
public export
renderTickCloses : (r : TickResult shape) ->
                   lastChar (renderChars r) = Just (closeChar shape)
renderTickCloses (EventsResult events) =
  lastSnoc ('[' :: unpack (joinBy "," events)) ']'
renderTickCloses (ErrorResult message) =
  lastAppSnoc ('{' :: unpack "\"error\":") (unpack message) '}'

-- --- digits, provably ------------------------------------------------------

||| The decimal digit characters. Total and digit-valued by construction.
public export
digitChar : Nat -> Char
digitChar 0 = '0'
digitChar 1 = '1'
digitChar 2 = '2'
digitChar 3 = '3'
digitChar 4 = '4'
digitChar 5 = '5'
digitChar 6 = '6'
digitChar 7 = '7'
digitChar 8 = '8'
digitChar 9 = '9'
digitChar _ = '0'

||| Quotient and remainder on division by ten, by structural recursion:
||| peel one successor at a time and carry at nine.
|||
||| `Data.Nat.divNat` cannot be used here. It is implemented via
||| `assert_total (integerToNat ...)`, so it is *non-covering* — `%default
||| total` rejects any proof that calls it, and it does not reduce during
||| elaboration, so nothing about its output is provable anyway. This
||| definition is structural, total, and reduces.
public export
divMod10 : Nat -> (Nat, Nat)
divMod10 Z     = (0, 0)
divMod10 (S n) =
  case divMod10 n of
    (q, 9) => (S q, 0)
    (q, r) => (q, S r)

||| Decimal rendering. The recursion is structural on `fuel`, so this is
||| total without an escape hatch; `natChars` supplies enough fuel.
public export
natCharsFuel : (fuel : Nat) -> Nat -> List Char
natCharsFuel Z     _ = ['0']
natCharsFuel (S f) n =
  case divMod10 n of
    (Z,   r) => [digitChar r]
    (S q, r) => natCharsFuel f (S q) ++ [digitChar r]

||| The decimal digits of a natural number.
public export
natChars : Nat -> List Char
natChars n = natCharsFuel (S n) n

-- --- NUL freedom -----------------------------------------------------------

||| No interior NUL: the property a C string must have.
public export
nulFree : List Char -> Bool
nulFree []        = True
nulFree (c :: cs) = if c == '\0' then False else nulFree cs

||| NUL-freedom is preserved by concatenation.
export
nulFreeApp : (xs, ys : List Char) ->
             nulFree xs = True -> nulFree ys = True -> nulFree (xs ++ ys) = True
nulFreeApp []        ys _  py = py
nulFreeApp (x :: xs) ys px py with (x == '\0')
  nulFreeApp (x :: xs) ys px py | True  = absurd px
  nulFreeApp (x :: xs) ys px py | False = nulFreeApp xs ys px py

||| Every digit character is non-NUL.
export
digitCharNulFree : (n : Nat) -> nulFree [digitChar n] = True
digitCharNulFree 0 = Refl
digitCharNulFree 1 = Refl
digitCharNulFree 2 = Refl
digitCharNulFree 3 = Refl
digitCharNulFree 4 = Refl
digitCharNulFree 5 = Refl
digitCharNulFree 6 = Refl
digitCharNulFree 7 = Refl
digitCharNulFree 8 = Refl
digitCharNulFree 9 = Refl
digitCharNulFree (S (S (S (S (S (S (S (S (S (S _)))))))))) = Refl

||| CHECKED: a decimal rendering never contains a NUL, for every fuel and
||| every number — so no tick counter can smuggle one into the wire.
export
natCharsFuelNulFree : (fuel : Nat) -> (n : Nat) ->
                      nulFree (natCharsFuel fuel n) = True
natCharsFuelNulFree Z     _ = Refl
natCharsFuelNulFree (S f) n with (divMod10 n)
  natCharsFuelNulFree (S f) n | (Z,   r) = digitCharNulFree r
  natCharsFuelNulFree (S f) n | (S q, r) =
    nulFreeApp (natCharsFuel f (S q)) [digitChar r]
               (natCharsFuelNulFree f (S q))
               (digitCharNulFree r)

||| CHECKED: `natChars` output is NUL-free.
public export
natCharsNulFree : (n : Nat) -> nulFree (natChars n) = True
natCharsNulFree n = natCharsFuelNulFree (S n) n

||| POSITIVE CONTROL: `natChars` really does render decimal. A NUL-freedom
||| theorem about a function that always returned `"0"` would be true and
||| useless.
public export
natCharsIsDecimal : natChars 123 = ['1', '2', '3']
natCharsIsDecimal = Refl

-- --------------------------------------------------------------------------
-- The snapshot wire, built on the digit and NUL machinery above.
-- --------------------------------------------------------------------------

||| A rendered snapshot without its closing brace. Keeping the brace as the
||| outermost concatenation is what makes both delimiter theorems below one
||| line each, instead of an associativity argument.
public export
snapshotBody : SnapshotResult -> List Char
snapshotBody (SnapshotOk snap) =
  '{' :: (unpack "\"format\":\"" ++
         (unpack snap.format ++
         (unpack "\",\"tick\":" ++ natChars (tick (state snap)))))
snapshotBody (SnapshotFail message) =
  '{' :: (unpack "\"error\":\"" ++ (unpack message ++ unpack "\""))

||| The rendered snapshot, as characters.
public export
renderSnapshotChars : SnapshotResult -> List Char
renderSnapshotChars r = snapshotBody r ++ ['}']

||| Wrap a snapshot result. Both cases open with '{' — see `SnapshotResult`.
|||
||| Defined through `renderSnapshotChars` so the theorems below are about
||| the function the ABI actually calls. It renders the tick counter with
||| `natChars` rather than `show`: `show` does not reduce during
||| elaboration, so a NUL-freedom proof about its output is unavailable.
||| The bytes are identical.
public export
renderSnapshot : SnapshotResult -> String
renderSnapshot = pack . renderSnapshotChars

||| CHECKED: every snapshot is a JSON object. Unlike the tick pair this is
||| not a discriminator — it is the honest statement that shape alone tells
||| a caller nothing here, so both forms must parse as objects.
public export
renderSnapshotIsObject : (r : SnapshotResult) ->
                         headChar (renderSnapshotChars r) = Just '{'
renderSnapshotIsObject (SnapshotOk _)   = Refl
renderSnapshotIsObject (SnapshotFail _) = Refl

||| CHECKED: and every snapshot closes the object it opened.
public export
renderSnapshotCloses : (r : SnapshotResult) ->
                       lastChar (renderSnapshotChars r) = Just '}'
renderSnapshotCloses r = lastSnoc (snapshotBody r) '}'

||| CHECKED: the snapshot `idap_ghost_lobby_snapshot_json` actually returns
||| contains no interior NUL, so it is a valid C string. Stated about the
||| form `Exports` produces — `SnapshotOk (snapshotOf st)` — because that
||| is the one whose format field is a literal; an arbitrary
||| `RuntimeSnapshot` carries an arbitrary `String` and no such claim holds.
public export
snapshotOfIsNulFree : (st : RunState) ->
                      nulFree (renderSnapshotChars (SnapshotOk (snapshotOf st))) = True
snapshotOfIsNulFree st =
  nulFreeApp (snapshotBody (SnapshotOk (snapshotOf st))) ['}']
    (nulFreeApp (unpack "\"format\":\"")
       (unpack (formatTag RuntimeV3) ++
         (unpack "\",\"tick\":" ++ natChars (tick st)))
       Refl
       (nulFreeApp (unpack (formatTag RuntimeV3))
          (unpack "\",\"tick\":" ++ natChars (tick st))
          Refl
          (nulFreeApp (unpack "\",\"tick\":") (natChars (tick st))
             Refl (natCharsNulFree (tick st)))))
    Refl

public export
ownSnapshot : SnapshotResult -> OwnedWire
ownSnapshot r = MkOwnedWire '{' (renderSnapshot r)
