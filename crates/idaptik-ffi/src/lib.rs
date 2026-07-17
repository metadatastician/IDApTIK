//! C-ABI surface over [`idaptik_core`].
//!
//! This is the stable boundary that non-Rust code binds to: the **Zig** FFI
//! bridge for APIs, and the **Idris2** models that specify the ABI contracts
//! (ADR-0001). Keep this layer thin — it owns only pointer/lifetime handling and
//! delegates every decision to `idaptik-core`.
//!
//! Regenerate the C header with cbindgen (`just ffi-header`, or from this
//! crate's directory):
//! ```text
//! cbindgen --output include/idaptik.h
//! ```
//!
//! Edition 2024 requires the `unsafe(...)` wrapper on `#[no_mangle]`.

use idaptik_core::scenario::{
    Buttons, Command, DifficultyId, GhostLobbySim, RunConfig, fold, ghost_lobby,
};
use idaptik_core::{Network, demo_network};
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Opaque owner of a [`Network`]. Consumers hold this as an opaque pointer and
/// only touch it through the functions below.
pub struct NetworkHandle(Network);

/// Build the demonstration network. The caller owns the returned pointer and
/// must release it with [`idap_network_free`].
#[unsafe(no_mangle)]
pub extern "C" fn idap_demo_network() -> *mut NetworkHandle {
    Box::into_raw(Box::new(NetworkHandle(demo_network())))
}

/// Free a network previously returned by this library. Passing null is a no-op.
///
/// # Safety
/// `ptr` must be a pointer returned by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_network_free(ptr: *mut NetworkHandle) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}

/// Number of devices in the network. Returns 0 for a null pointer.
///
/// # Safety
/// `ptr` must be null or a valid pointer from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_network_device_count(ptr: *const NetworkHandle) -> usize {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*ptr }.0.len()
}

// --- Ghost Lobby ----------------------------------------------------------
//
// JSON-in/JSON-out over the existing `Command`/`Event` serde surface, so the
// ABI stays four functions wide no matter how the gameplay vocabulary grows.
// Every gameplay decision stays in `idaptik-core`; this layer owns only
// pointers, lifetimes and UTF-8.

/// Opaque owner of a [`GhostLobbySim`] run plus the persistent held-button set.
///
/// The sim consumes one folded `TickInput` per 60 Hz frame, and held movement
/// buttons persist *across* ticks (`SetButton` presses stay down until released
/// — see how the TUI's `InputState::sample` feeds `GhostLobbySim::tick`). The
/// handle carries that held set so FFI callers speak the same per-tick
/// `Command` stream the Elixir session layer and the TUI already share, and
/// [`idaptik_core::scenario::fold`] collapses it exactly as they do.
pub struct GhostLobbyHandle {
    sim: GhostLobbySim,
    held: Buttons,
}

/// Allocate an owned, NUL-terminated C string. Returns null if `s` cannot be
/// represented (interior NUL — serde_json output never contains one, since JSON
/// escapes control characters).
fn owned_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Build the `{"error": "..."}` sentinel string returned for recoverable
/// failures, so callers can tell an error report from a JSON events array by
/// shape alone.
fn error_json(msg: &str) -> *mut c_char {
    owned_c_string(serde_json::json!({ "error": msg }).to_string())
}

/// Start a Ghost Lobby run: the bundled scenario definition, full motion, the
/// given seed and difficulty. The caller owns the returned pointer and must
/// release it with [`idap_ghost_lobby_free`].
///
/// `difficulty` is a NUL-terminated string: `"story"`, `"standard"` or
/// `"operator"` (case-insensitive). Returns null — never panics — if
/// `difficulty` is null, not UTF-8 or not one of those three, or if the
/// bundled definition fails validation.
///
/// The startup events (`RunStarted`, `SeedAnnounced`) are queued inside the
/// sim; the first [`idap_ghost_lobby_tick_json`] returns them ahead of its own
/// events.
///
/// # Safety
/// `difficulty` must be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_ghost_lobby_new(
    seed: u32,
    difficulty: *const c_char,
) -> *mut GhostLobbyHandle {
    if difficulty.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(text) = unsafe { CStr::from_ptr(difficulty) }.to_str() else {
        return std::ptr::null_mut();
    };
    let Ok(difficulty) = text.parse::<DifficultyId>() else {
        return std::ptr::null_mut();
    };
    let cfg = RunConfig {
        difficulty,
        reduced_motion: false,
    };
    match catch_unwind(|| GhostLobbySim::new(ghost_lobby(), cfg, seed)) {
        Ok(Ok(sim)) => Box::into_raw(Box::new(GhostLobbyHandle {
            sim,
            held: Buttons::default(),
        })),
        _ => std::ptr::null_mut(),
    }
}

/// Advance the run by exactly one fixed 60 Hz frame and return that tick's
/// events as an owned JSON string. Free it with [`idap_string_free`].
///
/// JSON contract (the same wire the Elixir session layer and the TUI share):
///
/// - **In** — `commands_json` is a JSON array of `Command` values for this
///   tick, e.g. `[{"cmd":"SetButton","button":"Right","down":true},
///   {"cmd":"Jump"}]`, or `[]` for an idle frame. The stream is folded into one
///   `TickInput`: `SetButton` mutates the handle's persistent held set (held
///   buttons stay down across later ticks until released), edges and
///   immediates apply this tick in stream order.
/// - **Out** — on success, a JSON array of the tick's `Event` values (the
///   first tick is prefixed with the queued startup events). On a recoverable
///   failure (null/invalid `commands_json`, or an unexpected panic in the sim,
///   which is contained here rather than crossing the boundary) an
///   `{"error":"..."}` object. Null only when `ptr` is null or allocation
///   fails.
///
/// # Safety
/// `ptr` must be null or a live pointer from [`idap_ghost_lobby_new`], not
/// freed, and not used concurrently from another thread. `commands_json` must
/// be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_ghost_lobby_tick_json(
    ptr: *mut GhostLobbyHandle,
    commands_json: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    if commands_json.is_null() {
        return error_json("commands_json is null; pass \"[]\" for an idle tick");
    }
    let Ok(text) = unsafe { CStr::from_ptr(commands_json) }.to_str() else {
        return error_json("commands_json is not valid UTF-8");
    };
    let commands: Vec<Command> = match serde_json::from_str(text) {
        Ok(commands) => commands,
        Err(e) => {
            return error_json(&format!(
                "commands_json is not a JSON array of commands: {e}"
            ));
        }
    };
    let handle = unsafe { &mut *ptr };
    // The sim's tick path is documented panic-free; this contains any
    // regression as an error sentinel instead of unwinding (UB) or aborting.
    let result = catch_unwind(AssertUnwindSafe(|| {
        let input = fold(&commands, &mut handle.held);
        serde_json::to_string(&handle.sim.tick(&input))
    }));
    match result {
        Ok(Ok(json)) => owned_c_string(json),
        Ok(Err(e)) => error_json(&format!("events did not serialise: {e}")),
        Err(_) => error_json("the simulation tick panicked; the run state may be inconsistent"),
    }
}

/// Serialise a full, restorable `RuntimeSnapshot` of the run at the current
/// tick (format tag `idaptik-ghost-lobby-runtime-v2`) as an owned JSON string.
/// Free it with [`idap_string_free`]. Returns an `{"error":"..."}` object on a
/// recoverable failure; null only when `ptr` is null or allocation fails.
///
/// # Safety
/// `ptr` must be null or a live pointer from [`idap_ghost_lobby_new`], not
/// freed, and not used concurrently from another thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_ghost_lobby_snapshot_json(
    ptr: *const GhostLobbyHandle,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let handle = unsafe { &*ptr };
    match catch_unwind(AssertUnwindSafe(|| {
        serde_json::to_string(&handle.sim.snapshot())
    })) {
        Ok(Ok(json)) => owned_c_string(json),
        Ok(Err(e)) => error_json(&format!("snapshot did not serialise: {e}")),
        Err(_) => error_json("snapshot panicked"),
    }
}

/// Free a Ghost Lobby run previously returned by [`idap_ghost_lobby_new`].
/// Passing null is a no-op.
///
/// # Safety
/// `ptr` must be null or a pointer returned by this library and not already
/// freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_ghost_lobby_free(ptr: *mut GhostLobbyHandle) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}

/// Free a string previously returned by this library ([`idap_ghost_lobby_tick_json`],
/// [`idap_ghost_lobby_snapshot_json`]). Passing null is a no-op.
///
/// # Safety
/// `s` must be null or a string returned by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idap_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_network_roundtrips_through_the_abi() {
        let handle = idap_demo_network();
        assert!(!handle.is_null());
        // SAFETY: handle came straight from idap_demo_network above.
        let count = unsafe { idap_network_device_count(handle) };
        assert_eq!(count, 6);
        unsafe { idap_network_free(handle) };
    }

    #[test]
    fn null_is_handled() {
        assert_eq!(unsafe { idap_network_device_count(std::ptr::null()) }, 0);
        unsafe { idap_network_free(std::ptr::null_mut()) }; // no-op, must not crash
    }

    // --- Ghost Lobby ------------------------------------------------------

    use idaptik_core::scenario::tuning::ActionKind;
    use idaptik_core::scenario::{Button, PivotTarget};

    /// Take ownership of an ABI-returned string, freeing it through the ABI.
    unsafe fn take_string(p: *mut c_char) -> String {
        assert!(!p.is_null(), "expected an owned string, got null");
        let s = unsafe { CStr::from_ptr(p) }
            .to_str()
            .expect("ABI strings are UTF-8")
            .to_owned();
        unsafe { idap_string_free(p) };
        s
    }

    #[test]
    fn ghost_lobby_full_run_over_the_abi_matches_an_in_process_run() {
        const SEED: u32 = 1337;

        // A per-tick command script exercising held buttons (persist across
        // ticks), edges, and immediates — typed once, serialised for the ABI
        // side so both runs consume the identical wire stream.
        let mut script: Vec<Vec<Command>> = vec![Vec::new(); 240];
        script[1] = vec![Command::SetButton {
            button: Button::Right,
            down: true,
        }];
        script[30] = vec![Command::Jump];
        script[60] = vec![Command::Pivot {
            target: PivotTarget::Bridge,
        }];
        script[90] = vec![Command::Uplink {
            kind: ActionKind::Camera,
        }];
        script[150] = vec![
            Command::SetButton {
                button: Button::Right,
                down: false,
            },
            Command::Interact,
        ];
        script[200] = vec![Command::Unpivot];

        // In-process reference run (do not drain the startup events: the ABI
        // run's first tick returns them ahead of its own, per the docs).
        let cfg = RunConfig::standard();
        let mut reference =
            GhostLobbySim::new(ghost_lobby(), cfg, SEED).expect("bundled definition is valid");
        let mut held = Buttons::default();

        // ABI run.
        let difficulty = CString::new("standard").expect("no interior NUL");
        let handle = unsafe { idap_ghost_lobby_new(SEED, difficulty.as_ptr()) };
        assert!(!handle.is_null(), "construction over the ABI failed");

        for (tick, commands) in script.iter().enumerate() {
            let wire = serde_json::to_string(commands).expect("commands serialise");
            let wire_c = CString::new(wire).expect("no interior NUL");
            let abi_events =
                unsafe { take_string(idap_ghost_lobby_tick_json(handle, wire_c.as_ptr())) };

            let input = fold(commands, &mut held);
            let expected = serde_json::to_string(&reference.tick(&input)).expect("serialises");
            assert_eq!(abi_events, expected, "tick {tick} diverged across the ABI");
        }

        // The first tick surfaced the queued startup events.
        // (Checked via the reference equality above; spot-check the snapshot.)
        let abi_snapshot = unsafe { take_string(idap_ghost_lobby_snapshot_json(handle)) };
        let expected_snapshot =
            serde_json::to_string(&reference.snapshot()).expect("snapshot serialises");
        assert_eq!(abi_snapshot, expected_snapshot, "final snapshots diverged");

        let parsed: serde_json::Value =
            serde_json::from_str(&abi_snapshot).expect("snapshot is JSON");
        assert_eq!(
            parsed["format"], "idaptik-ghost-lobby-runtime-v2",
            "snapshot carries the v2 format tag GhostLobbySim::restore requires"
        );

        unsafe { idap_ghost_lobby_free(handle) };
    }

    #[test]
    fn ghost_lobby_startup_events_arrive_on_the_first_tick() {
        let difficulty = CString::new("operator").expect("no interior NUL");
        let handle = unsafe { idap_ghost_lobby_new(7, difficulty.as_ptr()) };
        assert!(!handle.is_null());
        let idle = CString::new("[]").expect("no interior NUL");
        let events = unsafe { take_string(idap_ghost_lobby_tick_json(handle, idle.as_ptr())) };
        let parsed: serde_json::Value = serde_json::from_str(&events).expect("events are JSON");
        let kinds: Vec<&str> = parsed
            .as_array()
            .expect("events are a JSON array")
            .iter()
            .filter_map(|e| e.get("event")?.as_str())
            .collect();
        assert!(kinds.contains(&"RunStarted"), "got {kinds:?}");
        assert!(kinds.contains(&"SeedAnnounced"), "got {kinds:?}");
        unsafe { idap_ghost_lobby_free(handle) };
    }

    /// Assert `p` is the `{"error": ...}` sentinel and return the message.
    unsafe fn expect_error(p: *mut c_char) -> String {
        let s = unsafe { take_string(p) };
        let v: serde_json::Value = serde_json::from_str(&s).expect("sentinel is JSON");
        v.as_object()
            .and_then(|o| o.get("error"))
            .and_then(|e| e.as_str())
            .unwrap_or_else(|| panic!("expected an error sentinel, got {s}"))
            .to_owned()
    }

    #[test]
    fn ghost_lobby_bad_arguments_become_error_sentinels_not_ub() {
        // Construction failures return null.
        assert!(unsafe { idap_ghost_lobby_new(1, std::ptr::null()) }.is_null());
        let bogus = CString::new("nightmare").expect("no interior NUL");
        assert!(unsafe { idap_ghost_lobby_new(1, bogus.as_ptr()) }.is_null());
        let not_utf8 = CString::new(vec![0xFF, 0xFE]).expect("no interior NUL");
        assert!(unsafe { idap_ghost_lobby_new(1, not_utf8.as_ptr()) }.is_null());

        // Null handles return null, and freeing null is a no-op.
        let idle = CString::new("[]").expect("no interior NUL");
        assert!(
            unsafe { idap_ghost_lobby_tick_json(std::ptr::null_mut(), idle.as_ptr()) }.is_null()
        );
        assert!(unsafe { idap_ghost_lobby_snapshot_json(std::ptr::null()) }.is_null());
        unsafe { idap_ghost_lobby_free(std::ptr::null_mut()) };
        unsafe { idap_string_free(std::ptr::null_mut()) };

        // Bad tick input on a live handle: error sentinels, and the run survives.
        let difficulty = CString::new("standard").expect("no interior NUL");
        let handle = unsafe { idap_ghost_lobby_new(1, difficulty.as_ptr()) };
        assert!(!handle.is_null());

        let msg = unsafe { expect_error(idap_ghost_lobby_tick_json(handle, std::ptr::null())) };
        assert!(msg.contains("null"), "got {msg}");

        let garbage = CString::new("not json at all").expect("no interior NUL");
        unsafe { expect_error(idap_ghost_lobby_tick_json(handle, garbage.as_ptr())) };

        let wrong_shape = CString::new(r#"{"cmd":"Jump"}"#).expect("no interior NUL");
        unsafe { expect_error(idap_ghost_lobby_tick_json(handle, wrong_shape.as_ptr())) };

        let unknown_cmd = CString::new(r#"[{"cmd":"HackThePlanet"}]"#).expect("no interior NUL");
        unsafe { expect_error(idap_ghost_lobby_tick_json(handle, unknown_cmd.as_ptr())) };

        // A failed tick applied nothing: the handle still ticks normally.
        let events = unsafe { take_string(idap_ghost_lobby_tick_json(handle, idle.as_ptr())) };
        let parsed: serde_json::Value = serde_json::from_str(&events).expect("events are JSON");
        assert!(parsed.is_array(), "expected an events array, got {events}");

        unsafe { idap_ghost_lobby_free(handle) };
    }
}
