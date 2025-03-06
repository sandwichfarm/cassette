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

    // Get the @cassette dependency from build.zig.zon
    const cassette_dep = b.dependency("cassette", .{
        .target = target,
        .optimize = optimize,
    });

    // Build the main executable for dependency reasons only.
    // We do NOT install this artifact since our custom build step will invoke zig build-exe
    // with the proper "--no-entry" flag.
    const exe = b.addExecutable(.{
        .name = "incrementer",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = false,
        .single_threaded = true,
    });

    // Link the @cassette static library.
    exe.linkLibrary(cassette_dep.artifact("cassette"));

    // Do NOT install the executable as a build artifact.
    // b.installArtifact(exe);

    // Custom build step: run an external command to produce the Wasm file.
    // This command calls zig build-exe with the proper flags (--no-entry) to avoid requiring an entry symbol.
    const build_step = b.step("build-wasm", "Build WebAssembly module with no entry");
    build_step.dependOn(&exe.step);
    build_step.makeFn = struct {
        fn run(step: *std.Build.Step, prog_node: std.Progress.Node) !void {
            const child_node = prog_node.start("Building WebAssembly module", 1);
            defer child_node.end();

            // The command-line invocation explicitly passes --no-entry.
            const result = try std.process.Child.run(.{
                .allocator = step.owner.allocator,
                .argv = &[_][]const u8{
                    "zig",                 "build-exe",
                    "-o",                  "zig-out/bin/incrementer.wasm",
                    "src/main.zig",        "--target",
                    "wasm32-freestanding", "--no-entry",
                    "--export=increment",
                },
            });
            if (result.stderr.len > 0) {
                try step.addError("Build failed: {s}", .{result.stderr});
            }
        }
    }.run;
}
