||| Handle ownership as the header's `# Safety` sections describe it in
||| prose: one live reference, freed exactly once, never used after.
||| Linear types make that prose a machine-checked discipline.
module Idaptik.Abi.Handles

import Idaptik.Abi.CABI
import Idaptik.Abi.Run

%default total

||| The two opaque handle kinds the header declares.
public export
data HandleKind = NetworkKind | GhostLobbyKind

||| The states of a handle across its lifetime.
public export
data HandleState = Live | Freed

||| The demonstration network as the model sees it: opaque but for the one
||| observation `idap_network_device_count` makes of it.
public export
record NetworkModel where
  constructor MkNetworkModel
  deviceCount : Nat

||| What a live handle of each kind owns.
public export
Payload : HandleKind -> Type
Payload NetworkKind     = NetworkModel
Payload GhostLobbyKind  = RunState

||| Opaque handles, indexed by lifetime state. A live handle owns its
||| payload; a freed handle owns nothing and offers nothing.
public export
data Handle : HandleKind -> HandleState -> Type where
  LiveHandle  : (payload : Payload kind) -> Handle kind Live
  FreedHandle : Handle kind Freed

||| CHECKED: `Freed` and `Live` are distinct states. No value of one is a
||| value of the other; no operation turns a freed handle live again.
public export
freedNotLive : Freed = Live -> Void
freedNotLive Refl impossible

||| The transition every `*_free` function in the header performs: consume
||| the one linear reference to a live handle, leave a freed one. For a
||| linear client, double-free needs two references it cannot have, and
||| use-after-free needs a live reference that no longer exists — both are
||| type errors, not safety review items.
public export
free : (1 h : Handle kind Live) -> Handle kind Freed
free (LiveHandle _) = FreedHandle

||| The null-is-a-no-op rule shared by `idap_network_free` and
||| `idap_ghost_lobby_free`.
public export
freeNullable : (1 p : CNullable (Handle kind Live)) -> CNullable (Handle kind Freed)
freeNullable CNull        = CNull
freeNullable (CSome h)    = CSome (free h)

||| A freed handle is spent: the model offers no observation of it, and the
||| only thing left to do is discard the reference.
public export
discardFreed : (1 p : CNullable (Handle kind Freed)) -> ()
discardFreed CNull            = ()
discardFreed (CSome FreedHandle) = ()
