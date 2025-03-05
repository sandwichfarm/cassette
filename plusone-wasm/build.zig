const std = @import("std");

pub fn build(b: *std.Build) void {
    // Standard release options
    const optimize = b.standardOptimizeOption(.{});

    // Target wasm32-unknown-unknown
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
        .abi = .none,
    });

    const obj = b.addObject(.{
        .name = "incrementer",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    b.installArtifact(obj);
} 