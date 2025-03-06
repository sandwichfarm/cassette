const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{});
    const target = std.zig.CrossTarget{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
        .abi = .none,
    };

    // Add a custom build option for no entry
    const no_entry = true; // Set this flag as needed

    // Step to generate the describe function using cassette
    const generate_describe = b.addExecutable(.{
        .name = "generate_describe",
        .root_source_file = b.path("src/auto_describe.zig"),
        .target = b.getTarget(target),
        .optimize = optimize,
    });
    if (no_entry) {
        generate_describe.addLinkerFlag("-fno-entry");
    }
    const run_generate_describe = b.addRunArtifact(generate_describe);

    const test_step = b.step("test", "Run describe artifact");
    test_step.dependOn(&run_generate_describe.step);
    b.default_step.dependOn(test_step);

    // Main executable
    const exe = b.addExecutable(.{
        .name = "incrementer",
        .root_source_file = b.path("src/main.zig"),
        .target = b.getTarget(target),
        .optimize = optimize,
    });
    if (no_entry) {
        exe.addLinkerFlag("-fno-entry");
    }

    // Ensure the describe function is generated before building the main executable
    exe.step.dependOn(&run_generate_describe.step);

    b.installArtifact(exe);

    // Custom step to manually build with -fno-entry
    const build_step = b.step("build-wasm", "Build WebAssembly module with no entry");
    build_step.dependOn(&exe.step);
    build_step.run().args = &[_][]const u8{
        "zig",                "build-exe",           "src/main.zig",
        "-target",            "wasm32-freestanding", "-fno-entry",
        "--export=increment",
    };
}
