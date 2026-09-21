# ABI Authority — the canonical Zig unified-api-adapter API

Status: **authority located and pinned** — this resolves the conformance
blocker recorded in
[issue #103](https://github.com/metadatastician/IDApTIK/issues/103)
("did not locate a canonical Zig unified Hexadeca API repository/specification").

## The authority

The canonical Zig unified-api-adapter surface is **`hyperpolymath/hypatia`'s
Zig FFI library** (`ffi/zig/`), the estate-wide canonical implementation the
2026-09 estate rename (Hexadeca-Connector → UnifiedApiAdapter, 48 merged PRs)
consolidated on. There is no separate "Hexadeca spec" to find: hypatia *is*
the specification, and its connector wire contract is the ABI truth every
estate consumer (this repository included) conforms to.

## Pinned digests

Historical pin (the digest-pinned revision of record for audits):

| File @ `2bbd3b2` | sha256 |
|---|---|
| `ffi/connectors.json` | `16878b5b50145d860c9717bb035a001f5841aad20eb682308279e9063a44c15e` |
| `ffi/zig/src/hexadeca.zig` (renamed, see below) | `9e43876ae926567f6e0180a9da83a64bcb07222fde5347ef059a1bb0ecb56f22` |
| `src/Hypatia/ABI/Types.idr` | `b04478e836754334c474e4f09317e88344b95c884cac4fc591bdb50157faf187` |
| `clients/rust/hypatia-client/src/connector.rs` | `6c12e56e4c35954ee5eaab81a72c9f460b71e966364eeb0a9ee5e675702538c8` |

Current canonical revision: `hyperpolymath/hypatia@bcb8bb3730c2520117e565d80d5638ff384c1222`
(main), where the estate-wide rename has landed:

| File @ main | sha256 |
|---|---|
| `ffi/zig/src/unified-api-adapter.zig` | `f9a0b6fa9158ac7055d2d68f39a126800d8724a5c171bb2d35994e60dc3b5676` |
| `ffi/connectors.json` (unchanged by the rename) | `16878b5b50145d860c9717bb035a001f5841aad20eb682308279e9063a44c15e` |

`2bbd3b2` predates the rename: its `ffi/zig/src/hexadeca.zig` is the same
surface now published as `ffi/zig/src/unified-api-adapter.zig` (the rename
also applied the estate identifier mapping inside the file, which is why the
digest differs; the connector wire contract did **not** change).

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

`CONNECTOR_COUNT = 16`; `portFor = base + id + 1`. Connector ids, names, and
the wire format are byte-identical across the estate — the rename changed
*names of identifiers and paths only*, never ids or wire bytes.

## How IDApTIK consumes this

The Zig adapter (`crates/idaptik-ffi/zig-adapter/`) implements IDApTIK's C
boundary (`include/idaptik.h`, 8 exports) and its conformance fixtures pin
against the digests above. CI re-derives the digests from the pinned
revisions and rejects drift — see
`.github/workflows/abi-conformance.yml` and
`crates/idaptik-ffi/zig-adapter/test/conformance_test.zig`.
