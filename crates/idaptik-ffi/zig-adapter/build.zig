// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Build for the IDApTIK Zig FFI adapter (issue #103).
//
//   zig build test         — adapter unit tests (no library needed)
//   zig build conformance  — conformance harness against the Rust cdylib
//                            (build it first: cargo build -p idaptik-ffi;
//                            -Dlibdir=<dir> if the .so is not at the
//                            default ../../../target/debug)
//   zig build fuzz         — adversarial sweep over the same boundary
//
// -Dlibdir is the ONLY library knob. There is no IDAPTIK_FFI_LIB: nothing
// in this file or the harness reads an environment variable.

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const mod = b.addModule("idaptik_ffi_adapter", .{
        .root_source_file = b.path("src/idaptik_ffi_adapter.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Unit tests live in the module's root file.
    const unit_tests = b.addTest(.{ .root_module = mod });
    const run_unit = b.addRunArtifact(unit_tests);
    const test_step = b.step("test", "Run adapter unit tests");
    test_step.dependOn(&run_unit.step);

    // Conformance harness: imports the adapter module, links the Rust cdylib.
    const libdir = b.option([]const u8, "libdir", "directory containing libidaptik_ffi.so") orelse "../../../target/debug";
    const conf_mod = b.createModule(.{
        .root_source_file = b.path("test/conformance_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    conf_mod.addLibraryPath(.{ .cwd_relative = libdir });
    conf_mod.linkSystemLibrary("idaptik_ffi", .{});
    conf_mod.addImport("idaptik_ffi_adapter", mod);
    const conformance = b.addTest(.{ .root_module = conf_mod });
    const run_conf = b.addRunArtifact(conformance);
    const conf_step = b.step("conformance", "Run the ABI conformance harness (needs the Rust cdylib)");
    conf_step.dependOn(&run_conf.step);

    // Fuzz sweep: adversarial inputs through the live cdylib (issue #103).
    const fuzz_mod = b.createModule(.{
        .root_source_file = b.path("test/fuzz_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    fuzz_mod.addImport("idaptik_ffi_adapter", mod);
    fuzz_mod.addLibraryPath(.{ .cwd_relative = libdir });
    fuzz_mod.linkSystemLibrary("idaptik_ffi", .{});
    const fuzz = b.addTest(.{ .root_module = fuzz_mod });
    const run_fuzz = b.addRunArtifact(fuzz);
    const fuzz_step = b.step("fuzz", "Adversarial fuzz sweep against the cdylib");
    fuzz_step.dependOn(&run_fuzz.step);
}
