# UMS package contract ownership

IDApTIK owns gameplay meaning. UMS may author and validate source material, but
the game publishes the vocabulary and accepts or rejects the compiled artifact.
The v1 public contract is under `contracts/idaptik/v1/`.

Every listed gameplay definition is game-owned:

| Definition | Authoritative game location |
|---|---|
| Scenario format, rooms, doors, cameras, hide spots, props, objectives | `scenario::definition` |
| Floors, portals and traversal | `scenario::building`, `scenario::floor_graph` |
| Actors and composition | `scenario::actor` |
| Actions, tuning, difficulty and scoring | `scenario::tuning`, `scenario::outcome` |
| Grounded network topology | `netsim`, derived `scenario::floor_graph` |
| Network actions and physical effects | `netsim::effect`, `scenario::agents`, `scenario::event` |
| Snapshots and restoration | `scenario::snapshot`, `GhostLobbySim::restore` |
| Package syntax, compatibility and loading | `package`, `contracts/idaptik/v1` |

| Concern | Owner | Evidence |
|---|---|---|
| Package syntax and version | IDApTIK | `package.schema.json`, `package::GamePackage` |
| Scenario syntax and vocabulary | IDApTIK | `scenario::ScenarioDefinition` and its nested game types |
| Semantic validation | IDApTIK | stable validation IDs returned by `ScenarioDefinition::validate` |
| Profile-specific generation | UMS IDApTIK profile | `profiles/idaptik/v1/ghost-lobby.ums.json`, `ums-profiles` |
| Taxonomy mapping | UMS profile against IDApTIK terms | `contract.json#taxonomy_terms` |
| Migrations | IDApTIK | a new contract major/versioned loader; v1 has no migration yet |
| Runtime loading | IDApTIK | `package::load_package` |
| Runtime execution, snapshots and replay | IDApTIK | `package::run_package`, `GhostLobbySim` |

The authoritative Ghost Lobby scenario fixture lives at
`contracts/idaptik/v1/fixtures/ghost-lobby-scenario.json` and is compiled into
the game. UMS reads that artifact when producing the proving package; it does
not maintain another Rust `LevelData`, `ScenarioDefinition`, action enum, or
objective enum.

The package loader checks the format and contract versions, game/profile
compatibility, scenario ID, taxonomy coverage, actor bindings, scheduled
commands, deterministic seed, snapshot format and declared event guarantees.
It then runs the existing scenario semantic validator, including rooms and
spatial references, doors, cameras, objectives, action tuning, difficulty,
scoring and address capacity.

The current package envelope carries the Ghost Lobby scenario plus the command
stream used by the vertical slice. Floors and the grounded physical/network
graph remain derived game runtime structures. A future contract version may
publish building packages separately; v1 does not pretend that migration or
loader exists.
