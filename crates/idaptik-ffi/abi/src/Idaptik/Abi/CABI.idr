||| The C vocabulary of the `idaptik` header: nullable pointers, strings
||| that are fit to cross the boundary, and the ownership discipline the
||| header's own `# Safety` notes spell out in prose.
|||
||| This module models the *types*; the per-function contracts live in
||| `Idaptik.Abi.Exports`, one declaration per header export.
module Idaptik.Abi.CABI

import Data.So

%default total

||| Nullable C pointers. `CNull` is C `NULL`; a non-null pointer holds its
||| value linearly, so it can be consumed exactly once — the pointer
||| arithmetic of ownership, as types.
public export
data CNullable : Type -> Type where
  CNull : CNullable a
  CSome : (1 x : a) -> CNullable a

||| Interior NUL test. A C string is NUL-terminated, so a string handed in
||| or returned across the boundary must not embed NUL itself.
export
hasInteriorNul : String -> Bool
hasInteriorNul s = any (\c => c == '\0') (unpack s)

||| A string fit to cross the C boundary: Idris Strings are UTF-8, and the
||| auto-proof rules out the interior NUL that would truncate it in C.
public export
record CString where
  constructor MkCString
  text : String
  {auto 0 nulFree : So (not (hasInteriorNul text))}

||| Validate an arbitrary string for boundary fitness. Nothing is lost:
||| the string is rejected exactly when it could not survive the crossing.
public export
mkCString : (s : String) -> Maybe CString
mkCString s = case choose (not (hasInteriorNul s)) of
  Left oh  => Just (MkCString s {nulFree = oh})
  Right _  => Nothing

||| The three difficulties `idap_ghost_lobby_new` accepts.
public export
data Difficulty = Story | Standard | Operator

asciiLowerChar : Char -> Char
asciiLowerChar c = if c >= 'A' && c <= 'Z' then chr (ord c + 32) else c

asciiLower : String -> String
asciiLower = pack . map asciiLowerChar . unpack

||| The header accepts `\"story\"`, `\"standard\"` or `\"operator\"`
||| case-insensitively; everything else yields a null handle.
public export
parseDifficulty : String -> Maybe Difficulty
parseDifficulty s = case asciiLower s of
  "story"    => Just Story
  "standard" => Just Standard
  "operator" => Just Operator
  _          => Nothing

||| A string the library returned and the caller must release with
||| `idap_string_free`. The only modelled way to consume one is that free.
public export
data Owned : Type where
  MkOwned : (content : String) -> Owned

||| Model of `idap_string_free`: passing null is a no-op, and a non-null
||| owned string is consumed exactly once — a linear client can neither
||| double-free it nor leak it.
public export
stringFree : (1 s : CNullable Owned) -> ()
stringFree CNull                = ()
stringFree (CSome (MkOwned _))  = ()
