module Main

import Idaptik.Abi.CABI
import Idaptik.Abi.Exports

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
