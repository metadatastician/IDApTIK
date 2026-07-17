//! Task 9: the infiltrator's vantage follows their body, and a camera that has
//! been looped or unpowered stops seeing. Task 10: a filled trace ends the run,
//! whichever agent's trace it was. Task 11: the support envelope pays for the
//! hacker's reach as well as the room the infiltrator stands in. Task 14: the
//! floor's two winning lines are both playable, and the peer inside can reach the
//! peer outside by the same rules that let the peer outside reach in.

mod common;
use common::Runner;
use idaptik_core::netsim::{AgentSession, Effect, resolve};
use idaptik_core::scenario::agents::apply_effects;
use idaptik_core::scenario::command::{Button, Buttons, Command, PivotTarget};
use idaptik_core::scenario::common::{FailReason, Outcome};
use idaptik_core::scenario::constants as c;
use idaptik_core::scenario::event::Event;
use idaptik_core::scenario::{
    ActionKind, DifficultyId, DoorId, GhostLobbySim, RuntimeSnapshot, camera_node_id, door_node_id,
    floor_graph, ghost_lobby, inside_vantage, light_node_id,
};

/// A world-x that parks the agent squarely inside the hallway camera's cone. The
/// camera watches 292..495 in room coordinates and sweeps by at most 25 either
/// way, so a centre of 394 (this x plus the 14px room offset) is inside the cone
/// at every phase of the sweep rather than only at the lucky ones.
const HALL_CONE_X: f64 = 380.0;

/// Comfortably past the kitchen/hall door at x=270, so that reaching it is proof
/// the body crossed rather than merely leaned on the frame.
const INTO_THE_HALL_X: f64 = 330.0;

/// Pivot the hacker into the building's maintenance bridge, which is what opens a
/// route from the van to the floor's fixtures. Borrowed from `mechanics_b`:
/// driven as the real `Command::Pivot` (one tick), the same path a replay takes.
fn pivot_in(r: &mut Runner) {
    r.step(&[Command::Pivot {
        target: PivotTarget::Bridge,
    }]);
    assert!(
        r.saw(|e| matches!(e, Event::PivotOpened { .. })),
        "the van can reach the maintenance bridge"
    );
}

fn hold(button: Button) -> Command {
    Command::SetButton { button, down: true }
}

/// A snapshot of a standard run with the agent parked in the hallway camera's
/// cone and the floor otherwise untouched: no accumulated detection, no lockout,
/// nothing looped.
///
/// The body is placed rather than walked in, for two reasons. Walking there would
/// itself accumulate detection on the way, and these tests must measure a rise
/// from a clean slate. And the sim deliberately offers no `state_mut` (mutable
/// access goes through the command stream), so a snapshot is the only public seam
/// through which a test can seed `dead_nodes`, which no uplink on this floor can
/// reach.
fn parked_snapshot() -> RuntimeSnapshot {
    let mut snap = Runner::standard().sim.snapshot();
    snap.state.player.x = HALL_CONE_X;
    snap
}

/// A runner resumed from `snap`.
fn resume(snap: RuntimeSnapshot) -> Runner {
    Runner {
        sim: GhostLobbySim::restore(ghost_lobby(), snap).expect("the snapshot restores"),
        held: Buttons::default(),
        log: Vec::new(),
    }
}

#[test]
fn the_infiltrators_vantage_follows_them_across_rooms() {
    let mut r = Runner::standard();
    let first = r.sim.state().agents.infiltrator.vantage().entry_ip;
    assert!(
        r.sim
            .state()
            .agents
            .infiltrator
            .is_local(r.sim.graph(), &light_node_id("kitchen")),
        "they spawn owning the room they spawn in"
    );

    // Walk until the room changes. Every room boundary on this floor is a door,
    // and a closed door pins the agent's centre to its own side, so the hallway is
    // only enterable once the hacker has routed the kitchen/hall door open.
    pivot_in(&mut r);
    r.step(&[
        Command::Uplink {
            kind: ActionKind::Door,
        },
        hold(Button::Right),
    ]);
    for _ in 0..240 {
        if r.sim.state().player.x > INTO_THE_HALL_X {
            break;
        }
        r.step(&[]);
    }
    assert!(
        r.sim.state().player.x > INTO_THE_HALL_X,
        "the body must actually reach the hallway for this test to mean anything"
    );

    let second = r.sim.state().agents.infiltrator.vantage().entry_ip;
    assert_ne!(first, second, "the vantage must follow the body");
    assert_eq!(
        Some(second),
        inside_vantage(r.sim.graph(), "hall").map(|v| v.entry_ip),
        "and it must be the hallway's vantage in particular"
    );

    // And what they own locally has moved with them.
    assert!(
        r.sim
            .state()
            .agents
            .infiltrator
            .is_local(r.sim.graph(), &light_node_id("hall"))
    );
    assert!(
        !r.sim
            .state()
            .agents
            .infiltrator
            .is_local(r.sim.graph(), &light_node_id("kitchen")),
        "the room behind them is no longer theirs"
    );
}

#[test]
fn the_parked_agent_is_squarely_in_the_hall_cameras_cone() {
    // The two tests below assert that a camera does not detect. That is worth
    // nothing unless this same fixture, left alone, does detect: without this
    // control they would pass on a camera that was never going to see anybody.
    let mut r = resume(parked_snapshot());
    assert_eq!(
        r.sim.state().camera_detection,
        0.0,
        "the fixture starts on a clean slate"
    );
    for _ in 0..20 {
        r.step(&[]);
    }
    assert!(
        r.sim.state().camera_detection > 0.0,
        "the hall camera must see the parked agent"
    );
}

#[test]
fn a_looped_camera_does_not_detect() {
    // Looped by the hacker actually hacking it, not by a poked flag: the point of
    // the task is that the hack the hacker paid for buys what it promises.
    let mut r = resume(parked_snapshot());
    pivot_in(&mut r);
    r.step(&[Command::Uplink {
        kind: ActionKind::Camera,
    }]);
    assert!(
        r.sim.state().camera_looped[0] > 0.0,
        "the uplink must land on the camera watching the room they stand in"
    );

    let before = r.sim.state().camera_detection;
    for _ in 0..20 {
        r.step(&[]);
    }
    assert_eq!(
        r.sim.state().camera_detection,
        before,
        "a looped camera is blind"
    );
    assert_eq!(
        r.sim.state().stats.camera_detections,
        0,
        "and it flags nothing"
    );
}

#[test]
fn an_unpowered_camera_does_not_detect() {
    let mut snap = parked_snapshot();
    snap.state.dead_nodes.insert(camera_node_id(0));
    let mut r = resume(snap);

    let before = r.sim.state().camera_detection;
    for _ in 0..20 {
        r.step(&[]);
    }
    assert_eq!(
        r.sim.state().camera_detection,
        before,
        "a dead camera is blind"
    );
    assert_eq!(
        r.sim.state().stats.camera_detections,
        0,
        "and it flags nothing"
    );
}

//## Task 10: the trace fills and the run ends

/// Far more hacks than the threshold can survive, so a session that somehow stops
/// tracing fails the assertion below rather than spinning the test for ever.
const TRACE_HACK_CEILING: usize = 1_000;

/// A snapshot whose `pick`ed session has been worked until its trace trips.
///
/// The trace is filled on the snapshot's own session rather than through the
/// uplink: reaching the threshold takes upwards of a hundred hacks, which the
/// action cooldowns would spread across a run far longer than the one under test,
/// and the point being tested is the ending rather than the arithmetic that
/// arrives at it. As Task 9 found, a snapshot is the public seam for this; the sim
/// deliberately offers no `state_mut` (mutable access goes through the command
/// stream).
///
/// The session must pivot in before it can work: cold from its own vantage the
/// hall lights answer `NoRoute`, the trace never moves, and the loop would never
/// terminate. Both the pivot and every hack are asserted for that reason.
fn traced_snapshot(pick: impl Fn(&mut RuntimeSnapshot) -> &mut AgentSession) -> RuntimeSnapshot {
    let base = Runner::standard();
    let graph = base.sim.graph();
    let mut snap = base.sim.snapshot();
    pick(&mut snap)
        .ssh(graph, "bridge.local")
        .expect("both vantages can reach the maintenance bridge");

    // The hall lights are off both agents' home segments, so hacking them always
    // traces; a local fixture would be free and the loop would never end.
    let hall = light_node_id("hall");
    for _ in 0..TRACE_HACK_CEILING {
        if pick(&mut snap).traced() {
            break;
        }
        pick(&mut snap)
            .hack(graph, &hall)
            .expect("the hall lights answer from the bridge");
    }
    assert!(
        pick(&mut snap).traced(),
        "the fixture must actually trip the trace, or the test below proves nothing"
    );
    snap
}

/// The run must end as `Traced`, with the debrief and the log line to match.
fn assert_traced_ending(mut r: Runner) {
    r.step(&[]);
    assert!(r.sim.is_ended());
    let d = r.sim.debrief().expect("the run has a debrief");
    assert!(!d.success);
    assert_eq!(d.reason, Outcome::Traced);
    assert!(r.saw(|e| matches!(
        e,
        Event::MissionFailed {
            reason: FailReason::Traced
        }
    )));
}

#[test]
fn a_tripped_trace_ends_the_run_as_traced() {
    let snap = traced_snapshot(|s| &mut s.state.agents.hacker);
    assert_traced_ending(resume(snap));
}

//## Task 11: the hacker's reach costs the link

/// Long enough for the support envelope to settle, and short enough that nothing
/// else has begun to move it. `approach` is linear rather than asymptotic, so the
/// envelope lands exactly on its target and stays: from the opening 1.0 down to
/// the deepest target under test is 0.23, and `SUPPORT_APPROACH / 60` a tick
/// closes that in 12. Meanwhile the hall camera cannot flag the parked agent, and
/// so cannot charge the alert term, until `STANDARD_CAMERA_LOCK` (0.78s, 47
/// ticks) has passed. Thirty ticks sits comfortably between the two.
const SETTLE_TICKS: u64 = 30;

/// The settled support of a run parked in the hallway, with the hacker either at
/// their van or one pivot deep. The hallway is the fixture rather than the
/// kitchen because its base support sits strictly inside the clamp: at the
/// kitchen's 1.0 the ceiling would mask the very term under test.
fn settled_support(pivot: bool) -> f64 {
    let mut r = resume(parked_snapshot());
    if pivot {
        pivot_in(&mut r);
    }
    assert_eq!(
        r.sim.state().agents.hacker.hops(),
        u32::from(pivot),
        "the fixture must be at the depth it claims"
    );
    r.idle(SETTLE_TICKS);
    assert!(
        !r.sim.is_ended(),
        "the run must still be live to be measured"
    );
    r.sim.state().support
}

#[test]
fn the_two_routes_the_floor_offers_cost_the_link_differently() {
    // The point of the whole term. The floor offers the hacker a one-pivot line
    // through the maintenance bridge and a two-pivot line out through the ISP to
    // the substation, and depth is the only thing in the pressure model that
    // tells the two apart. Both are walked through their real hosts rather than
    // poked onto the stack, so a topology that collapsed the two depths together
    // would fail here rather than quietly flatten the choice.
    //
    // The kitchen is the fixture because no camera watches it: the alert term
    // stays out of the arithmetic, leaving the base and the hops.
    let bridge = {
        let mut r = Runner::standard();
        pivot_in(&mut r);
        assert_eq!(r.sim.state().agents.hacker.hops(), 1);
        r.idle(SETTLE_TICKS);
        r.sim.state().support
    };
    let upstream = {
        let mut r = Runner::standard();
        r.step(&[Command::Pivot {
            target: PivotTarget::IspOps,
        }]);
        r.step(&[Command::Pivot {
            target: PivotTarget::GridJump,
        }]);
        assert_eq!(
            r.sim.state().agents.hacker.hops(),
            2,
            "the upstream line is genuinely two pivots deep"
        );
        r.idle(SETTLE_TICKS);
        r.sim.state().support
    };

    assert_eq!(bridge, c::ROOM_KITCHEN_SUPPORT - c::SUPPORT_HOP_PEN);
    assert_eq!(upstream, c::ROOM_KITCHEN_SUPPORT - 2.0 * c::SUPPORT_HOP_PEN);
    assert!(upstream < bridge, "the longer reach must cost the more");
    // Riskier, but not unplayable: from a room this strong the upstream line
    // still clears the isolation gate. Task 14's acceptance tests are the arbiter
    // of that balance.
    assert!(upstream > c::ISO_GATE);
}

#[test]
fn reaching_deeper_frays_support() {
    let shallow = settled_support(false);
    let deep = settled_support(true);

    // Parked in the hallway the room base is the only live term: no ping, full
    // bandwidth, no alert yet, unhidden, the lights up. So a shallow settle
    // landing exactly on the room base proves both that the approach has
    // finished and that nothing else is dragging the envelope about.
    assert_eq!(
        shallow,
        c::ROOM_HALL_SUPPORT,
        "the shallow fixture must settle on the room base alone"
    );
    assert!(deep < shallow, "a deeper reach must cost more support");
    assert!(
        (shallow - deep - c::SUPPORT_HOP_PEN).abs() < 1e-9,
        "one pivot must cost exactly one hop penalty: {shallow} - {deep}"
    );
    // Both sit strictly inside the clamp, so that difference is the term itself
    // and not an artefact of the floor or the ceiling.
    assert!(deep > c::SUPPORT_CLAMP_MIN && shallow < 1.0);
}

#[test]
fn a_pivot_costs_the_link_only_while_the_hacker_stands_on_it() {
    // The penalty is a property of where the hacker is now, not of where they
    // have been: backing out of the pivot must hand the envelope back. This one
    // is parked in the kitchen rather than the hallway because it must settle
    // twice, which outlasts the hall camera's lock; no camera watches the
    // kitchen, so the alert term stays out of the arithmetic throughout.
    let mut r = Runner::standard();
    pivot_in(&mut r);
    r.idle(SETTLE_TICKS);
    let deep = r.sim.state().support;
    assert_eq!(
        deep,
        c::ROOM_KITCHEN_SUPPORT - c::SUPPORT_HOP_PEN,
        "one pivot costs one hop penalty against the kitchen's base"
    );

    // The stack is popped through the snapshot seam rather than through Task 14's
    // `Command::Unpivot`, and deliberately: this test is about what the hop
    // penalty costs while it is stood on, so it must not also depend on the verb
    // that pops it. `Command::Unpivot` is exercised by the net smoke tests.
    let mut snap = r.sim.snapshot();
    assert!(snap.state.agents.hacker.exit(), "the pivot pops");
    let mut r = resume(snap);
    assert_eq!(r.sim.state().agents.hacker.hops(), 0);
    r.idle(SETTLE_TICKS);
    assert!(
        r.sim.state().support > deep,
        "coming home must restore the support the reach was costing"
    );
    assert_eq!(r.sim.state().support, c::ROOM_KITCHEN_SUPPORT);
}

#[test]
fn the_infiltrators_tripped_trace_ends_the_run_too() {
    // The symmetry is the whole point: both players are the same kind of peer, so
    // whoever is traced, the run ends. Typical play has only the hacker reaching
    // in, which is exactly why a check written for the hacker alone would pass
    // every other test in this suite and still be wrong.
    let snap = traced_snapshot(|s| &mut s.state.agents.infiltrator);
    assert_traced_ending(resume(snap));
}

//## Task 12: the trace threshold is a per-difficulty knob

#[test]
fn each_difficulty_carries_its_own_trace_threshold() {
    let def = ghost_lobby();
    let story = &def.difficulty[&DifficultyId::Story];
    let standard = &def.difficulty[&DifficultyId::Standard];
    let operator = &def.difficulty[&DifficultyId::Operator];
    assert!(story.trace_threshold > standard.trace_threshold);
    assert!(standard.trace_threshold > operator.trace_threshold);
}

//## Task 14: both winning lines are playable, and the symmetry is real

/// Generous headroom on the door's routing delay, which is rolled in
/// `0.22..=0.72`s (44 ticks at the top of the range) and never longer.
const ROUTE_TICKS: u64 = 120;

/// The door the given uplink routed, learnt from the event the route raised
/// rather than assumed. Which door the uplink picks is `action_target`'s
/// business (the one nearest the body); asking the event means these tests
/// cannot quietly drift onto a different door than the one the sim chose.
fn routed_door(r: &Runner) -> DoorId {
    r.log
        .iter()
        .find_map(|e| match e {
            Event::DoorRouted { door, .. } => Some(door.clone()),
            _ => None,
        })
        .expect("the uplink must have routed a door")
}

/// How far open `door` currently is.
fn door_open(r: &Runner, door: &DoorId) -> f64 {
    r.sim
        .state()
        .doors
        .iter()
        .find(|d| d.id == *door)
        .map(|d| d.open)
        .expect("the routed door exists in the runtime state")
}

#[test]
fn the_hacker_can_pop_a_door_remotely_under_trace() {
    // The building line, end to end, through the keys a player actually has: the
    // pivot command `p` fires, then the door uplink `2`. Before Task 14 there was
    // no way to say the first of those, so the second was denied for ever.
    let mut r = Runner::standard();
    r.step(&[Command::Pivot {
        target: PivotTarget::Bridge,
    }]);
    assert!(
        r.saw(|e| matches!(e, Event::PivotOpened { hops: 1, .. })),
        "the pivot command must actually land the hacker on the bridge"
    );

    r.step(&[Command::Uplink {
        kind: ActionKind::Door,
    }]);
    assert!(
        r.saw(|e| matches!(
            e,
            Event::UplinkAction {
                kind: ActionKind::Door
            }
        )),
        "the uplink must land now, where cold from the van it was denied on route"
    );
    let door = routed_door(&r);

    // The hold is routed, not instant: the door is still shut this tick, and only
    // opens once the rolled delay has run down. Asserting the shut state first is
    // what stops the test passing on a door that was never closed.
    assert_eq!(door_open(&r, &door), 0.0, "the route is still pending");
    for _ in 0..ROUTE_TICKS {
        if door_open(&r, &door) > 0.0 {
            break;
        }
        r.step(&[]);
    }

    assert!(door_open(&r, &door) > 0.0, "the door opened");
    assert!(
        r.sim.state().agents.hacker.trace_fraction() > 0.0,
        "and it cost trace"
    );
}

#[test]
fn the_infiltrator_opens_the_same_door_locally_for_free() {
    // The other half of the pair above, and the whole of the "local actuation is
    // cheap" claim: the same fixture, the same verb, no trace at all. The body
    // must be standing in the room the door's node lives on, which is why it is
    // walked there rather than asserted from the spawn.
    let mut r = Runner::standard();
    r.step(&[Command::Pivot {
        target: PivotTarget::Bridge,
    }]);
    r.step(&[
        Command::Uplink {
            kind: ActionKind::Door,
        },
        hold(Button::Right),
    ]);
    let door = routed_door(&r);

    // Every room boundary on this floor sits under a door, so the hall is only
    // enterable once the hacker has routed this one open.
    for _ in 0..240 {
        if r.sim.state().player.x > INTO_THE_HALL_X {
            break;
        }
        r.step(&[]);
    }
    assert!(
        r.sim.state().player.x > INTO_THE_HALL_X,
        "the body must actually cross for the door to be theirs to own"
    );
    assert_eq!(
        r.sim.state().agents.infiltrator.trace_fraction(),
        0.0,
        "walking is not hacking: the infiltrator has not traced yet"
    );

    // The sim offers no `state_mut`, deliberately (mutable access goes through
    // the command stream), so the snapshot is the public seam through which the
    // infiltrator's own hack
    // is driven -- exactly as Tasks 9 and 10 found.
    let graph = r.sim.graph();
    let mut snap = r.sim.snapshot();
    let target = door_node_id(door);
    assert!(
        snap.state.agents.infiltrator.is_local(graph, &target),
        "the door they are standing at must be on their own segment"
    );
    let effects = snap
        .state
        .agents
        .infiltrator
        .hack(graph, &target)
        .expect("the door is in the room they stand in");

    assert_eq!(snap.state.agents.infiltrator.trace_fraction(), 0.0, "free");
    assert!(effects.contains(&Effect::DoorHeld(target)));
}

#[test]
fn the_upstream_power_line_is_a_winnable_strategy() {
    // The second line, and the one the plan's `g` key exists for: out through the
    // ISP, on to the grid jump host, and only from that depth does the substation
    // answer. Both pivots are driven as the real commands the keys emit, so a
    // keymap that could not express the second hop would fail here.
    let mut r = Runner::standard();
    r.step(&[Command::Pivot {
        target: PivotTarget::IspOps,
    }]);
    r.step(&[Command::Pivot {
        target: PivotTarget::GridJump,
    }]);
    assert_eq!(
        r.sim.state().agents.hacker.hops(),
        2,
        "the upstream line is genuinely two pivots deep"
    );

    // No uplink action targets the substation, so the hack itself is driven
    // through the session seam. See the report: the strategy's mechanics are
    // whole, but the plan binds no key to pull this particular lever.
    let graph = r.sim.graph();
    let mut snap = r.sim.snapshot();
    let effects = snap
        .state
        .agents
        .hacker
        .hack(graph, "substation")
        .expect("the utility segment is open from the grid jump host");
    let cfg = r.sim.config();
    let def = ghost_lobby();
    apply_effects(&def, &mut snap.state, cfg, &effects);

    assert!(snap.state.lights_flicker > 0.0, "the floor went dark");
    assert!(
        snap.state.dead_nodes.contains(&camera_node_id(0)),
        "cameras died"
    );
    assert!(
        !snap.state.agents.hacker.traced(),
        "and the run is still alive"
    );
    // The run being alive is the whole point, so pin how much room is left rather
    // than merely that some is: two pivots and a hack must not cost a third of the
    // budget, or the line is a trap dressed as a choice. The whole line runs
    // through Strong hosts, whose security-scaled trace cost (40 a touch, divided
    // by the bounce depth) prices it at 73 of the 600-point budget.
    assert!(
        snap.state.agents.hacker.trace_fraction() < 0.15,
        "the upstream line must stay affordable: {}",
        snap.state.agents.hacker.trace_fraction()
    );
}

#[test]
fn the_infiltrator_can_reach_the_van_by_pivoting() {
    // The symmetry is real, not rhetorical: the peer inside can reach the peer
    // outside by exactly the rules that let the peer outside reach in.
    let def = ghost_lobby();
    let g = floor_graph(&def);
    let mut s = AgentSession::new(&g, inside_vantage(&g, "hall").unwrap(), 10_000);
    let ap = resolve(&g, "ap.local").expect("the van's AP resolves");
    assert!(!s.reachable(&g).contains(&ap), "not without pivoting");
    s.ssh(&g, "bridge.local")
        .expect("the infiltrator can pivot too");
    assert!(
        s.reachable(&g).contains(&ap),
        "and then the van is in reach"
    );
}
