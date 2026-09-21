// SPDX-License-Identifier: AGPL-3.0-or-later
//
//! Zig FFI adapter over the IDApTIK C boundary (`crates/idaptik-ffi/include/idaptik.h`).
//!
//! Issue #103: every declaration here corresponds one-for-one with the Idris2
//! model in `crates/idaptik-ffi/abi/src/Idaptik/Abi/Exports.idr` (field
//! `exportedFunctions`, header order) and with the header itself. The gate
//! script `scripts/zig_adapter_surface_check.sh` diffs all three surfaces and
//! rejects drift in either direction.
//!
//! Ownership mirrors the model:
//!  - `free` functions accept an optional handle; null is a no-op
//!    (model: `freeNullable` / `CNull`).
//!  - Tick/snapshot hand the handle back alongside the owned wire
//!    (model: `LPair (CNullable (Handle …)) (CNullable OwnedWire)`).
//!  - An owned wire string is consumed exactly once; `OwnedWire.deinit`
//!    traps on double-free in safe builds (model: linear `idap_string_free`).
//!  - Tick wire shape is discriminated by its first non-space byte:
//!    `[` = success events array, `{` = `{"error":"…\"}` object
//!    (model: `Idaptik.Abi.Json.renderTickIdentifiesShape`).

const std = @import("std");

/// The exported C surface, in header order. One entry per `idaptik.h`
/// function — kept in lockstep with `Exports.idr.exportedFunctions` by
/// `scripts/zig_adapter_surface_check.sh`.
pub const exported_functions = [_][]const u8{
    "idap_demo_network",
    "idap_network_free",
    "idap_network_device_count",
    "idap_ghost_lobby_new",
    "idap_ghost_lobby_tick_json",
    "idap_ghost_lobby_snapshot_json",
    "idap_ghost_lobby_free",
    "idap_string_free",
};

// --- raw C surface (mirrors idaptik.h; do not reorder) ----------------------

pub const NetworkHandle = opaque {};
pub const GhostLobbyHandle = opaque {};

pub const c = struct {
    pub extern fn idap_demo_network() ?*NetworkHandle;
    pub extern fn idap_network_free(ptr: ?*NetworkHandle) void;
    pub extern fn idap_network_device_count(ptr: ?*const NetworkHandle) usize;
    pub extern fn idap_ghost_lobby_new(seed: u32, difficulty: ?[*:0]const u8) ?*GhostLobbyHandle;
    pub extern fn idap_ghost_lobby_tick_json(ptr: ?*GhostLobbyHandle, commands_json: ?[*:0]const u8) ?[*:0]u8;
    pub extern fn idap_ghost_lobby_snapshot_json(ptr: ?*const GhostLobbyHandle) ?[*:0]u8;
    pub extern fn idap_ghost_lobby_free(ptr: ?*GhostLobbyHandle) void;
    pub extern fn idap_string_free(s: ?[*:0]u8) void;
};

// --- owned wire (mirrors CABI.OwnedWire) -------------------------------------

/// An ABI-owned NUL-terminated string, or null (model: `CNull`).
/// A live wire is consumed exactly once.
pub const OwnedWire = struct {
    ptr: ?[*:0]u8 = null,
    consumed: bool = false,

    pub fn isNull(self: *const OwnedWire) bool {
        return self.ptr == null;
    }

    pub fn slice(self: *const OwnedWire) ?[:0]const u8 {
        const p = self.ptr orelse return null;
        return std.mem.span(@as([*:0]const u8, @ptrCast(p)));
    }

    /// Model: `idap_string_free` / `consumeWire`. Null is a no-op; a live
    /// wire is released through the ABI exactly once.
    pub fn deinit(self: *OwnedWire) void {
        const p = self.ptr orelse return;
        if (self.consumed) {
            @panic("OwnedWire double-free: the Idris model makes this a type error");
        }
        self.consumed = true;
        c.idap_string_free(p);
    }
};

/// Model: the tick/snapshot `LPair` — the (possibly still-live) handle plus
/// the owned wire it produced.
pub fn Pair(comptime H: type) type {
    return struct { handle: H, wire: OwnedWire };
}

/// Model: `Idaptik.Abi.Json` — the first non-space byte identifies the shape.
pub const WireShape = enum { events_array, error_object, other };

pub fn wireShape(wire: []const u8) WireShape {
    for (wire) |ch| {
        switch (ch) {
            ' ' | '\t' | '\n' | '\r' => continue,
            '[' => return .events_array,
            '{' => return .error_object,
            else => return .other,
        }
    }
    return .other;
}

/// True when the wire is an `{"error":"…"}` object whose message contains
/// `needle`. Only meaningful for `.error_object` wires.
pub fn errorContains(wire: []const u8, needle: []const u8) bool {
    if (wireShape(wire) != .error_object) return false;
    return std.mem.indexOf(u8, wire, needle) != null;
}

// --- the adapter (one method per header function, header order) --------------

pub const Adapter = struct {
    /// Valid difficulties per the header contract (`"story"`, `"standard"`,
    /// `"operator"`, case-insensitive). Mirrors the model's `parseDifficulty`.
    pub const valid_difficulties = [_][]const u8{ "story", "standard", "operator" };

    pub fn isKnownDifficulty(name: []const u8) bool {
        for (valid_difficulties) |d| {
            if (std.ascii.eqlIgnoreCase(d, name)) return true;
        }
        return false;
    }

    /// Header: `struct NetworkHandle *idap_demo_network(void);`
    /// Model: `idap_demo_network : () -> CNullable (Handle NetworkKind Live)`.
    pub fn demoNetwork() ?*NetworkHandle {
        return c.idap_demo_network();
    }

    /// Header: `void idap_network_free(struct NetworkHandle *ptr);`
    /// Null is a no-op. Model: `idap_network_free` / `freeNullable`.
    pub fn networkFree(ptr: ?*NetworkHandle) void {
        c.idap_network_free(ptr);
    }

    /// Header: `uintptr_t idap_network_device_count(const struct NetworkHandle *ptr);`
    /// Returns 0 for null. Model: `idap_network_device_count`.
    pub fn networkDeviceCount(ptr: ?*const NetworkHandle) usize {
        return c.idap_network_device_count(ptr);
    }

    /// Header: `struct GhostLobbyHandle *idap_ghost_lobby_new(uint32_t seed, const char *difficulty);`
    /// Returns null — never panics — for a null or invalid difficulty.
    /// Model: `idap_ghost_lobby_new`.
    pub fn ghostLobbyNew(seed: u32, difficulty: ?[]const u8) ?*GhostLobbyHandle {
        const d = difficulty orelse return null;
        if (!isKnownDifficulty(d)) return null;
        const buf = std.heap.page_allocator.allocSentinel(u8, d.len, 0) catch return null;
        defer std.heap.page_allocator.free(buf);
        @memcpy(buf[0..d.len], d);
        return c.idap_ghost_lobby_new(seed, buf.ptr);
    }

    /// Header: `char *idap_ghost_lobby_tick_json(struct GhostLobbyHandle *ptr, const char *commands_json);`
    /// Advances exactly one 60 Hz frame and hands back the live handle plus
    /// the owned events/error wire. Model: `idap_ghost_lobby_tick_json`.
    pub fn ghostLobbyTickJson(
        handle: *GhostLobbyHandle,
        commands_json: ?[]const u8,
    ) Pair(*GhostLobbyHandle) {
        const raw: ?[*:0]u8 = blk: {
            const cj = commands_json orelse break :blk c.idap_ghost_lobby_tick_json(handle, null);
            const buf = std.heap.page_allocator.allocSentinel(u8, cj.len, 0) catch
                break :blk c.idap_ghost_lobby_tick_json(handle, null);
            defer std.heap.page_allocator.free(buf);
            @memcpy(buf[0..cj.len], cj);
            break :blk c.idap_ghost_lobby_tick_json(handle, buf.ptr);
        };
        return .{ .handle = handle, .wire = ownWire(raw) };
    }

    /// Header: `char *idap_ghost_lobby_snapshot_json(const struct GhostLobbyHandle *ptr);`
    /// Model: `idap_ghost_lobby_snapshot_json`.
    pub fn ghostLobbySnapshotJson(handle: *GhostLobbyHandle) Pair(*GhostLobbyHandle) {
        return .{ .handle = handle, .wire = ownWire(c.idap_ghost_lobby_snapshot_json(handle)) };
    }

    /// Header: `void idap_ghost_lobby_free(struct GhostLobbyHandle *ptr);`
    /// Null is a no-op. Model: `idap_ghost_lobby_free` / `freeNullable`.
    pub fn ghostLobbyFree(ptr: ?*GhostLobbyHandle) void {
        c.idap_ghost_lobby_free(ptr);
    }

    /// Header: `void idap_string_free(char *s);`
    /// Model: `idap_string_free`.
    pub fn stringFree(wire: *OwnedWire) void {
        wire.deinit();
    }

    fn ownWire(raw: ?[*:0]u8) OwnedWire {
        if (raw) |p| return .{ .ptr = p };
        // Null wire (model: `CNull` branch of `consumeWire`): deinit no-ops.
        return .{};
    }
};

test "surface table matches the header order" {
    try std.testing.expectEqual(@as(usize, 8), exported_functions.len);
    try std.testing.expectEqualStrings("idap_demo_network", exported_functions[0]);
    try std.testing.expectEqualStrings("idap_string_free", exported_functions[7]);
}

test "wire shape discrimination (model: renderTickIdentifiesShape)" {
    try std.testing.expectEqual(WireShape.events_array, wireShape("[{\"type\":\"RunStarted\"}]"));
    try std.testing.expectEqual(WireShape.error_object, wireShape("{\"error\":\"boom\"}"));
    try std.testing.expectEqual(WireShape.other, wireShape("garbage"));
    try std.testing.expect(errorContains("{\"error\":\"commands_json is null\"}", "commands_json is null"));
}

test "difficulty gate mirrors parseDifficulty" {
    try std.testing.expect(Adapter.isKnownDifficulty("story"));
    try std.testing.expect(Adapter.isKnownDifficulty("OPERATOR"));
    try std.testing.expect(!Adapter.isKnownDifficulty("nightmare"));
    try std.testing.expect(!Adapter.isKnownDifficulty(""));
}
