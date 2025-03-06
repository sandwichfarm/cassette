const std = @import("std");
const BuildError = error{CreateDirFailed};

const MAIN_FILE = "src/main.zig";

pub fn build(b: *std.Build) void {
    const build_step = b.step("build-wasm", "Build WebAssembly module");

    build_step.makeFn = buildWasm;

    b.getInstallStep().dependOn(build_step);
}

fn buildWasm(step: *std.Build.Step, prog_node: std.Progress.Node) !void {
    const child_node = prog_node.start("Building WebAssembly module", 1);
    defer child_node.end();

    var mkdir_cmd = [_][]const u8{
        "mkdir",
        "-p",
        "zig-out/bin",
    };
    const mkdir_result = try std.process.Child.run(.{
        .allocator = std.heap.page_allocator,
        .cwd = null,
        .argv = &mkdir_cmd,
    });
    if (mkdir_result.stderr.len > 0) {
        try step.addError("mkdir failed: {s}", .{mkdir_result.stderr});
    }

    var exe_argv = [_][]const u8{
        "zig",
        "build-exe",
        MAIN_FILE,
        "-target",
        "wasm32-freestanding",
        "-fno-entry",
        "--export=increment",
        "--export=decrement",
        "-femit-bin=zig-out/bin/counter.wasm",
    };
    const result = try std.process.Child.run(.{
        .allocator = std.heap.page_allocator,
        .cwd = null,
        .argv = &exe_argv,
    });
    if (result.stderr.len > 0) {
        try step.addError("Build failed: {s}", .{result.stderr});
    }
}
