// SPDX-License-Identifier: AGPL-3.0-or-later
//
//! Conformance harness for the IDApTIK C boundary (issue #103).
//!
//! Runs the fixture vectors in `fixtures/abi/conformance-vectors.json`
//! through the Zig adapter against the live Rust cdylib (linked at build
//! time — build it first: `cargo build -p idaptik-ffi`), and proves the
//! harness still rejects drift by asserting that every case in
//! `fixtures/abi/planted-failure.json` FAILS.
//!
//!   zig build conformance -Dlibdir=../../target/debug

const std = @import("std");
const adapter = @import("idaptik_ffi_adapter");
const c = adapter.c;

const Outcome = union(enum) {
    pass,
    fail: []const u8,
};

fn check(cond: bool, comptime fmt: []const u8, args: anytype) Outcome {
    if (cond) return .pass;
    return .{ .fail = std.fmt.allocPrint(std.heap.page_allocator, fmt, args) catch "OOM" };
}

fn cstring(alloc: std.mem.Allocator, s: []const u8) ?[*:0]const u8 {
    const buf = alloc.allocSentinel(u8, s.len, 0) catch return null;
    @memcpy(buf[0..s.len], s);
    return buf.ptr;
}

fn networkDeviceCount(expect_count: usize) Outcome {
    const h = c.idap_demo_network() orelse return .{ .fail = "idap_demo_network returned null" };
    const got = c.idap_network_device_count(h);
    c.idap_network_free(h);
    return check(got == expect_count, "device_count: expected {d}, got {d}", .{ expect_count, got });
}

fn networkDeviceCountNull() Outcome {
    const got = c.idap_network_device_count(null);
    c.idap_network_free(null); // must be a no-op, not a crash
    return check(got == 0, "null device_count: expected 0, got {d}", .{got});
}

fn ghostLobbyNew(seed: u32, difficulty: ?[]const u8, want_live: bool) Outcome {
    const alloc = std.heap.page_allocator;
    const d: ?[*:0]const u8 = if (difficulty) |s| cstring(alloc, s) orelse return .{ .fail = "OOM" } else null;
    const h = c.idap_ghost_lobby_new(seed, d);
    c.idap_ghost_lobby_free(h);
    return check((h != null) == want_live, "handle liveness: expected {s}, got {s}", .{
        if (want_live) "live" else "null",
        if (h != null) "live" else "null",
    });
}

fn tickCase(
    seed: u32,
    difficulty: []const u8,
    commands: ?[]const u8,
    expect_shape: ?[]const u8,
    contains_in_order: []const []const u8,
    must_not_contain: []const []const u8,
    error_contains: ?[]const u8,
) Outcome {
    const alloc = std.heap.page_allocator;
    const dbuf = alloc.allocSentinel(u8, difficulty.len, 0) catch return .{ .fail = "OOM" };
    defer alloc.free(dbuf);
    @memcpy(dbuf[0..difficulty.len], difficulty);
    const h = c.idap_ghost_lobby_new(seed, dbuf.ptr) orelse
        return .{ .fail = "ghost_lobby_new returned null for a valid difficulty" };
    defer c.idap_ghost_lobby_free(h);

    var pair = if (commands) |cj|
        adapter.Adapter.ghostLobbyTickJson(h, cj)
    else
        adapter.Adapter.ghostLobbyTickJson(h, null);
    defer pair.wire.deinit();

    const wire_opt = pair.wire.slice();
    if (expect_shape) |shape| {
        const w = wire_opt orelse return .{ .fail = "expected a wire, got null" };
        const shape_got: []const u8 = switch (adapter.wireShape(w)) {
            .events_array => "events_array",
            .error_object => "error_object",
            .other => "other",
        };
        if (!std.mem.eql(u8, shape_got, shape))
            return .{ .fail = std.fmt.allocPrint(alloc, "shape: expected {s}, got {s} (wire: {s})", .{ shape, shape_got, w[0..@min(w.len, 90)] }) catch "OOM" };
        if (error_contains) |needle| {
            if (!adapter.errorContains(w, needle))
                return .{ .fail = std.fmt.allocPrint(alloc, "error wire lacks \"{s}\": {s}", .{ needle, w[0..@min(w.len, 130)] }) catch "OOM" };
        }
        var cursor: usize = 0;
        for (contains_in_order) |needle| {
            const at = std.mem.indexOfPos(u8, w, cursor, needle) orelse
                return .{ .fail = std.fmt.allocPrint(alloc, "wire lacks {s} in order: {s}", .{ needle, w[0..@min(w.len, 130)] }) catch "OOM" };
            cursor = at + needle.len;
        }
        for (must_not_contain) |needle| {
            if (std.mem.indexOf(u8, w, needle) != null)
                return .{ .fail = std.fmt.allocPrint(alloc, "wire must not contain {s}", .{needle}) catch "OOM" };
        }
    } else if (wire_opt != null) {
        return .{ .fail = "expected null wire" };
    }
    return .pass;
}

fn determinismCase(seed: u32, difficulty: []const u8, commands: []const []const u8) Outcome {
    const alloc = std.heap.page_allocator;
    const dbuf = alloc.allocSentinel(u8, difficulty.len, 0) catch return .{ .fail = "OOM" };
    defer alloc.free(dbuf);
    @memcpy(dbuf[0..difficulty.len], difficulty);

    var runs: [2][][]const u8 = undefined;
    for (0..2) |run| {
        const h = c.idap_ghost_lobby_new(seed, dbuf.ptr) orelse return .{ .fail = "new returned null" };
        const wires = alloc.alloc([]const u8, commands.len) catch return .{ .fail = "OOM" };
        runs[run] = wires;
        for (commands, 0..) |cj, i| {
            var pair = adapter.Adapter.ghostLobbyTickJson(h, cj);
            const w = pair.wire.slice() orelse {
                c.idap_ghost_lobby_free(h);
                return .{ .fail = "tick returned null wire" };
            };
            wires[i] = alloc.dupe(u8, w) catch return .{ .fail = "OOM" };
            pair.wire.deinit();
        }
        c.idap_ghost_lobby_free(h);
    }
    defer for (0..2) |run| {
        for (runs[run]) |w| alloc.free(w);
        alloc.free(runs[run]);
    };
    for (commands, 0..) |_, i| {
        if (!std.mem.eql(u8, runs[0][i], runs[1][i]))
            return .{ .fail = std.fmt.allocPrint(alloc, "tick {d} diverged between identical runs", .{i}) catch "OOM" };
    }
    return .pass;
}

fn snapshotCase(seed: u32, difficulty: []const u8) Outcome {
    const alloc = std.heap.page_allocator;
    const dbuf = alloc.allocSentinel(u8, difficulty.len, 0) catch return .{ .fail = "OOM" };
    defer alloc.free(dbuf);
    @memcpy(dbuf[0..difficulty.len], difficulty);
    const h = c.idap_ghost_lobby_new(seed, dbuf.ptr) orelse return .{ .fail = "new returned null" };
    defer c.idap_ghost_lobby_free(h);
    var pair = adapter.Adapter.ghostLobbySnapshotJson(h);
    defer pair.wire.deinit();
    const w = pair.wire.slice() orelse return .{ .fail = "snapshot returned null" };
    // A snapshot is an object on both success and failure (the honest gap the
    // model records); it must never be an events array.
    return check(adapter.wireShape(w) != .events_array, "snapshot must be a JSON object, not an array", .{});
}

fn stringFreeNullNoop() Outcome {
    var wire = adapter.OwnedWire{};
    wire.deinit(); // null wire: no-op, must not crash
    return .pass;
}

extern "c" fn fopen(path: [*:0]const u8, mode: [*:0]const u8) ?*anyopaque;
extern "c" fn fread(ptr: [*]u8, size: usize, nmemb: usize, f: *anyopaque) usize;
extern "c" fn fseek(f: *anyopaque, off: c_long, whence: c_int) c_int;
extern "c" fn ftell(f: *anyopaque) c_long;
extern "c" fn fclose(f: *anyopaque) c_int;

fn readFileAllocC(alloc: std.mem.Allocator, path: []const u8) ![]u8 {
    const p = try alloc.allocSentinel(u8, path.len, 0);
    defer alloc.free(p);
    @memcpy(p[0..path.len], path);
    const f = fopen(p.ptr, "rb") orelse return error.FileNotFound;
    defer _ = fclose(f);
    _ = fseek(f, 0, 2); // SEEK_END
    const size: usize = @intCast(ftell(f));
    _ = fseek(f, 0, 0); // SEEK_SET
    const buf = try alloc.alloc(u8, size);
    errdefer alloc.free(buf);
    const got = fread(buf.ptr, 1, size, f);
    if (got != size) return error.ShortRead;
    return buf;
}

fn runSuite(path: []const u8, want: enum { pass, fail }) !usize {
    const alloc = std.heap.page_allocator;
    const text = try readFileAllocC(alloc, path);
    defer alloc.free(text);
    const parsed = try std.json.parseFromSlice(std.json.Value, alloc, text, .{});
    defer parsed.deinit();
    const cases = parsed.value.object.get("cases").?.array;
    var matched: usize = 0;
    for (cases.items) |item| {
        const id = item.object.get("id").?.string;
        const outcome = runCase(item.object);
        const failed = outcome == .fail;
        const wanted_failure = want == .fail;
        const ok = if (wanted_failure) failed else !failed;
        if (ok) {
            matched += 1;
            std.debug.print("  ok  {s}\n", .{id});
        } else {
            const reason = if (outcome == .fail) outcome.fail else "(expected failure did not occur)";
            std.debug.print("  FAIL {s}: {s}\n", .{ id, reason });
            return error.SuiteMismatch;
        }
    }
    return matched;
}

fn runCase(case: std.json.ObjectMap) Outcome {
    const alloc = std.heap.page_allocator;
    const kind = (case.get("kind") orelse jsonFail("kind")).string;
    const expect = (case.get("expect") orelse jsonFail("expect")).object;
    if (std.mem.eql(u8, kind, "network_device_count")) {
        return networkDeviceCount(@intCast(expect.get("device_count").?.integer));
    } else if (std.mem.eql(u8, kind, "network_device_count_null")) {
        return networkDeviceCountNull();
    } else if (std.mem.eql(u8, kind, "ghost_lobby_new")) {
        const seed: u32 = @intCast(case.get("seed").?.integer);
        const diff = case.get("difficulty").?.string;
        const want_live = std.mem.eql(u8, expect.get("handle").?.string, "live");
        return ghostLobbyNew(seed, diff, want_live);
    } else if (std.mem.eql(u8, kind, "tick")) {
        const seed: u32 = @intCast(case.get("seed").?.integer);
        const diff = case.get("difficulty").?.string;
        const commands: ?[]const u8 = if (case.get("commands")) |cv| cv.string else null;
        const shape: ?[]const u8 = if (expect.get("shape")) |sv| sv.string else null;
        var in_order: []const []const u8 = &.{};
        if (expect.get("contains_in_order")) |arr| {
            const items = alloc.alloc([]const u8, arr.array.items.len) catch return .{ .fail = "OOM" };
            for (arr.array.items, 0..) |it, i| items[i] = it.string;
            in_order = items;
        }
        var not_contain: []const []const u8 = &.{};
        if (expect.get("must_not_contain")) |arr| {
            const items = alloc.alloc([]const u8, arr.array.items.len) catch return .{ .fail = "OOM" };
            for (arr.array.items, 0..) |it, i| items[i] = it.string;
            not_contain = items;
        }
        return tickCase(seed, diff, commands, shape, in_order, not_contain, if (expect.get("error_contains")) |ev| ev.string else null);
    } else if (std.mem.eql(u8, kind, "tick_null")) {
        const seed: u32 = @intCast(case.get("seed").?.integer);
        const diff = case.get("difficulty").?.string;
        return tickCase(seed, diff, null, expect.get("shape").?.string, &.{}, &.{}, expect.get("error_contains").?.string);
    } else if (std.mem.eql(u8, kind, "determinism")) {
        const seed: u32 = @intCast(case.get("seed").?.integer);
        const diff = case.get("difficulty").?.string;
        const arr = case.get("commands_by_tick").?.array;
        const cmds = alloc.alloc([]const u8, arr.items.len) catch return .{ .fail = "OOM" };
        for (arr.items, 0..) |it, i| cmds[i] = it.string;
        return determinismCase(seed, diff, cmds);
    } else if (std.mem.eql(u8, kind, "snapshot")) {
        const seed: u32 = @intCast(case.get("seed").?.integer);
        return snapshotCase(seed, case.get("difficulty").?.string);
    } else if (std.mem.eql(u8, kind, "string_free_null")) {
        return stringFreeNullNoop();
    } else if (std.mem.eql(u8, kind, "owned_wire_double_free")) {
        const trap_enforced = @import("builtin").mode != .ReleaseFast;
        return check(trap_enforced, "double-free guard inactive in ReleaseFast", .{});
    }
    return .{ .fail = "unknown case kind" };
}

fn jsonFail(field: []const u8) noreturn {
    std.debug.panic("fixture missing field {s}", .{field});
}

test "conformance: vectors pass, planted failures are caught" {
    std.debug.print("\nconformance vectors:\n", .{});
    _ = try runSuite("../../../fixtures/abi/conformance-vectors.json", .pass);
    std.debug.print("planted failures (must be caught):\n", .{});
    _ = try runSuite("../../../fixtures/abi/planted-failure.json", .fail);
}
