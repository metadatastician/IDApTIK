# ABI Authority — hypatia, with the ABI in Idris2 and the FFI in Zig

Status: **authority ruled and pinned** — this resolves the conformance blocker
recorded in
[issue #103](https://github.com/metadatastician/IDApTIK/issues/103)
("did not locate a canonical Zig unified Hexadeca API repository/specification").

## The authority, and which layer is normative

The canonical ABI authority is **`hyperpolymath/hypatia`**, ruled by the estate
owner on 2026-09-21. There is no separate "Hexadeca spec" to find: hypatia *is*
the specification, and the estate-wide 2026-09 rename
(Hexadeca-Connector → UnifiedApiAdapter) consolidated on it.

The authority is **layered, and the layering is load-bearing**:

| Layer | Role | Where it lives in hypatia |
|---|---|---|
| **ABI** | **Normative.** The specification itself, written in Idris2. | `src/Hypatia/ABI/*.idr` |
| **FFI** | **Implementation.** Conforms to the ABI; is not itself the spec. | `ffi/zig/src/` |
| **Wire manifest** | The connector id/name table both layers agree on. | `ffi/connectors.json` |
| **BEAM boundary** | Supervised **SNIFs**, never raw NIFs, so a native fault is isolated from the BEAM. | *(none in hypatia today)* |

Conformance therefore runs in one direction: an implementation is checked
**against** the Idris2 ABI. The Zig FFI is the reference *implementation* of
that ABI, not the ground truth — a disagreement between the two is a defect in
the Zig, never a redefinition of the ABI.

## Pinned digests

Canonical revision: `hyperpolymath/hypatia@bcb8bb3730c2520117e565d80d5638ff384c1222` (main).

### Normative layer — the Idris2 ABI

| File @ `bcb8bb37` | sha256 |
|---|---|
| `src/Hypatia/ABI/Types.idr` | `a5f794b11cfbb38b9c38ba861af71d7cbd3e9b97c29a14973b212642d2f0d84e` |
| `src/Hypatia/ABI/FFI.idr` | `e7fb117bfe137e76232ddb220db82a389808a63d3eda29e3d47057052366c669` |
| `src/Hypatia/ABI/REST.idr` | `3d3bfe554afcfca22d5fa064621d9f6fb94e16a9b0a2a56b434ef5c66543813f` |
| `src/Hypatia/ABI/GraphQL.idr` | `a5122cfe2039fd1b06ab1e81fadb9d11b23418edd0b8bc25179d5546200914c5` |
| `src/Hypatia/ABI/GRPC.idr` | `c04165ae3426b7bde34d64d21277bbf5d0e0f66fdfa08ac71c9968df6888e5b4` |
| `src/Hypatia/ABI/RuleEngine.idr` | `a191ada02ed9534cd66eb2b3c8de45e354682358e8f9cec4b676b68788adc3bd` |

### Implementation layer — the Zig FFI and the wire manifest

| File @ `bcb8bb37` | sha256 |
|---|---|
| `ffi/zig/src/unified-api-adapter.zig` | `f9a0b6fa9158ac7055d2d68f39a126800d8724a5c171bb2d35994e60dc3b5676` |
| `ffi/connectors.json` | `33ee987eeb20283676e663bb6eacb5534ecb0da4bec344891aca08e7933be842` |

### Historical pin (the pre-rename revision of record, for audits)

| File @ `2bbd3b2` | sha256 |
|---|---|
| `ffi/connectors.json` | `16878b5b50145d860c9717bb035a001f5841aad20eb682308279e9063a44c15e` |
| `ffi/zig/src/hexadeca.zig` (renamed, see below) | `9e43876ae926567f6e0180a9da83a64bcb07222fde5347ef059a1bb0ecb56f22` |
| `src/Hypatia/ABI/Types.idr` | `b04478e836754334c474e4f09317e88344b95c884cac4fc591bdb50157faf187` |
| `clients/rust/hypatia-client/src/connector.rs` | `6c12e56e4c35954ee5eaab81a72c9f460b71e966364eeb0a9ee5e675702538c8` |

`2bbd3b2` predates the rename: its `ffi/zig/src/hexadeca.zig` is the same
surface now published as `ffi/zig/src/unified-api-adapter.zig`. The rename also
applied the estate identifier mapping inside the files, which is why those
digests differ.

`ffi/connectors.json` **did** change across the rename, contrary to what an
earlier draft of this document asserted. The diff is one line: the advisory
`_note` string was renamed (`hexadeca-connector` → `unified-api-adapter`, and
the guard test it names). **Connector ids, names and ordering are unchanged**,
so the wire contract is identical while the file digest is not. Both digests
are pinned above; do not assume a "documentation-only" rename leaves a file
byte-identical — re-derive it.

## The wire contract (what conformance means here)

From the pinned `ffi/connectors.json` (16 connectors, ids stable):

| id | connector | id | connector |
|----|-----------|----|-----------|
| 0 | grpc | 8 | trpc |
| 1 | graphql | 9 | capnproto |
| 2 | rest | 10 | soap |
| 3 | flatbuffers | 11 | verismbd-rest |
| 4 | bebop | 12 | bsp |
| 5 | jsonrpc | 13 | scip |
| 6 | websocket | 14 | ipfs |
| 7 | mqtt | 15 | arrow-flight |

`CONNECTOR_COUNT = 16`; `portFor = base + id + 1`. Connector ids, names and the
wire format are byte-identical across the estate — the rename changed *names of
identifiers and paths only*, never ids or wire bytes. **Wire ordering is
load-bearing: do not renumber.**

## How IDApTIK consumes this

IDApTIK mirrors the same layering:

| Layer | IDApTIK |
|---|---|
| ABI (normative, Idris2) | `crates/idaptik-ffi/abi/src/Idaptik/Abi/*.idr` |
| FFI (implementation, Zig) | `crates/idaptik-ffi/zig-adapter/` |
| C boundary | `crates/idaptik-ffi/src/lib.rs`, cbindgen → `include/idaptik.h` |
| BEAM boundary | *none — IDApTIK has no BEAM source and no NIF usage (measured 2026-09-21)* |

Should IDApTIK ever grow BEAM-native code, the estate rule is **supervised
SNIFs rather than raw NIFs** — a raw NIF executes inside the BEAM scheduler,
so a native fault fells the VM and destroys the supervision guarantees that
are the reason to be on the BEAM in the first place. The estate scanner is
`cadastra/tools/estate-migration-toolkit/scripts/find-nif-to-snif.sh`.

The Zig adapter implements IDApTIK's C boundary (8 exports) and its conformance
fixtures pin against the digests above. CI re-derives the digests from the
pinned revisions and rejects drift — see `.github/workflows/abi-conformance.yml`
and `crates/idaptik-ffi/zig-adapter/test/conformance_test.zig`.

## Known limitation — drift is detected, not prevented

The 16-connector enum currently agrees across Idris2, Zig and Rust **by hand**;
no code is generated from the normative ABI. The gate described above therefore
*detects* divergence after it is committed, rather than making it impossible.
Generating the Zig and Rust enums from the Idris2 ABI is the intended end state
and is tracked as a separate issue; until it lands, this document plus the
digest gate are the control.
