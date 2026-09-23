module Main

import Idaptik.Abi.CABI
import Idaptik.Abi.Exports
import Idaptik.Abi.Json

||| Runtime smoke for the idaptik-abi model (run by scripts/abi_model_check.sh):
||| execute the full linear client session and the rejection paths. The
||| typechecker has already proved the properties; this proves the model
||| also *runs* under the Chez backend.

sessionWith : String -> String -> String
sessionWith difficulty wire =
  case mkCString difficulty of
    Nothing => "smoke: mkCString rejected " ++ difficulty
    Just d  => case mkCString wire of
      Nothing => "smoke: mkCString rejected " ++ wire
      Just c  => demoSession 1234 d c

||| `Idaptik.Abi.Json.natChars` renders the snapshot tick counter. It exists
||| because `show : Nat -> String` elaborates to `primNumShow` and therefore
||| does NOT reduce, so no proof about a rendered numeral can be stated over
||| it. The theorems (`natCharsNulFree`, `natCharsIsDecimal`) are about
||| `natChars`; the claim that it emits the SAME BYTES as `show` is not a
||| theorem and must not be asserted as one. It is measured here.
natCharsAgrees : Nat -> Bool
natCharsAgrees n = pack (natChars n) == show n

||| The first value at which two renderings disagree, if any.
firstDisagreement : (Nat -> String) -> List Nat -> Maybe Nat
firstDisagreement _ []         = Nothing
firstDisagreement f (n :: ns)  =
  if f n == show n then firstDisagreement f ns else Just n

||| The census. Kept small enough that the structural `divMod10` stays cheap.
checkedRange : List Nat
checkedRange = [0 .. 2000] ++ [4095, 9999, 10000, 65535]

||| A renderer that is wrong by one trailing character. If the comparator
||| cannot catch this, agreement above means nothing.
bogusRender : Nat -> String
bogusRender n = pack (natChars n ++ ['0'])

main : IO ()
main = do
  -- Full session: new (case-insensitive difficulty), tick, snapshot,
  -- free run, free every owned string.
  putStrLn (sessionWith "STANDARD" "SetButton:Right:down;Jump")
  -- Idle frame: the empty command stream is a valid tick.
  putStrLn (sessionWith "STANDARD" "")
  -- Unknown difficulty: null handle, no panic.
  putStrLn (sessionWith "nightmare" "Jump")
  -- Interior NUL: the boundary validation rejects it before C ever sees it.
  putStrLn (case mkCString "stand\0ard" of
    Nothing => "smoke: interior NUL rejected"
    Just _  => "smoke: interior NUL accepted")
  -- Negative control FIRST: a comparator that cannot say no proves nothing.
  putStrLn (case firstDisagreement bogusRender checkedRange of
    Just _  => "smoke: natChars comparator rejects a wrong renderer"
    Nothing => "smoke: natChars comparator ACCEPTED a wrong renderer")
  -- The claim itself, with its denominator.
  putStrLn (case firstDisagreement (pack . natChars) checkedRange of
    Nothing => "smoke: natChars = show over " ++ show (length checkedRange)
                 ++ " values"
    Just n  => "smoke: natChars DISAGREES with show at " ++ show n)
