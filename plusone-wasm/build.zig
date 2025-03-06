const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{});
    const target = b.standardTargetOptions(.{
        .default_target = .{
            .cpu_arch = .wasm32,
            .os_tag = .freestanding,
            .abi = .none,
        },
    });

    const generate_describe = b.addExecutable(.{
        .name = "generate_describe",
        .root_source_file = b.path("src/auto_describe.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = false,
        .single_threaded = true,
    });
    const run_generate_describe = b.addRunArtifact(generate_describe);

    const test_step = b.step("test", "Run describe artifact");
    test_step.dependOn(&run_generate_describe.step);
    b.default_step.dependOn(test_step);

    const exe = b.addExecutable(.{
        .name = "incrementer",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = false,
        .single_threaded = true,
    });

    // Ensure the describe artifact is generated before building the main executable
    exe.step.dependOn(&run_generate_describe.step);
    b.installArtifact(exe);

    // Custom build step: run an external command via a custom run function.
    const build_step = b.step("build-wasm", "Build WebAssembly module with no entry");
    build_step.dependOn(&exe.step);
    build_step.makeFn = struct {
        fn run(step: *std.Build.Step, prog_node: std.Progress.Node) !void {
            const child_node = prog_node.start("Building WebAssembly module", 1);
            defer child_node.end();

            const result = try std.process.Child.run(.{
                .allocator = step.owner.allocator,
                .argv = &[_][]const u8{
                    "zig",                "build-exe",           "src/main.zig",
                    "-target",            "wasm32-freestanding", "-fno-entry",
                    "--export=increment",
                },
            });
            if (result.stderr.len > 0) try step.addError("Build failed: {s}", .{result.stderr});
        }
    }.run;
}
