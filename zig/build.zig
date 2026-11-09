const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // 1. Dynamic C-ABI shared library for Bun FFI (`bun:ffi`) and Rust FFI
    const lib_mod = b.createModule(.{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });

    const shared_lib = b.addLibrary(.{
        .linkage = .dynamic,
        .name = "yolo_zig",
        .root_module = lib_mod,
    });
    b.installArtifact(shared_lib);

    // 2. Static library for direct Rust linkage (`yolo-design-codegen` / `yolo-tokens`)
    const static_lib = b.addLibrary(.{
        .linkage = .static,
        .name = "yolo_zig_static",
        .root_module = lib_mod,
    });
    b.installArtifact(static_lib);

    // 3. Unit test suite
    const unit_tests = b.addTest(.{
        .root_module = lib_mod,
    });
    const run_unit_tests = b.addRunArtifact(unit_tests);
    const test_step = b.step("test", "Run mathematical, colorimetric, and SDF geometry tests");
    test_step.dependOn(&run_unit_tests.step);
}
