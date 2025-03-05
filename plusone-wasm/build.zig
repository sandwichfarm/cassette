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

    // Create a step to generate the describe function
    const generate_describe = b.addRunArtifact(b.addExecutable(.{
        .name = "generate_describe",
        .root_source_file = b.path("src/auto_describe.zig"),
        .target = target,
        .optimize = optimize,
    }));

    // Add the describe function to the main module
    const obj = b.addObject(.{
        .name = "incrementer",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Add the describe function to exports
    obj.addExport("describe", generate_describe);

    b.installArtifact(obj);
} 