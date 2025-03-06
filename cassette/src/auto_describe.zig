const std = @import("std");
const schema = @import("schema.zig");

/// Automatically generates a describe function for all exported functions in a module
pub fn generateDescribeFunction(comptime source_file: []const u8) fn () [*:0]const u8 {
    const ast = std.zig.Ast.parse(std.heap.page_allocator, source_file) catch @panic("Failed to parse source file");
    defer ast.deinit(std.heap.page_allocator);

    var schema_parts: []const u8 = "";
    var first = true;

    // Find all exported functions
    var i: usize = 0;
    while (ast.tokens.len > i) : (i += 1) {
        const token = ast.tokens.get(i);
        if (token.id == .keyword_export) {
            // Look for function declaration
            var j = i + 1;
            while (ast.tokens.len > j) : (j += 1) {
                const next_token = ast.tokens.get(j);
                if (next_token.id == .keyword_fn) {
                    // Found an exported function
                    const func_name = ast.tokenSlice(ast.tokens.get(j + 1));
                    const func_schema = generateFunctionSchema(ast, j);

                    if (first) {
                        schema_parts = std.fmt.comptimePrint("{{\"{s}\": {s}", .{ func_name, func_schema });
                        first = false;
                    } else {
                        schema_parts = std.fmt.comptimePrint("{s}, \"{s}\": {s}", .{ schema_parts, func_name, func_schema });
                    }
                    break;
                }
            }
        }
    }

    const final_schema = std.fmt.comptimePrint("{s}}}", .{schema_parts});

    return struct {
        pub fn describe() [*:0]const u8 {
            return schema.createWasmString(final_schema);
        }
    }.describe;
}

fn generateFunctionSchema(comptime ast: std.zig.Ast, start_index: usize) []const u8 {
    var i = start_index;
    var params: []const u8 = "";
    var first_param = true;

    // Find function parameters
    while (ast.tokens.len > i) : (i += 1) {
        const token = ast.tokens.get(i);
        if (token.id == .l_paren) {
            // Found parameter list start
            i += 1;
            while (ast.tokens.len > i) : (i += 1) {
                const param_token = ast.tokens.get(i);
                if (param_token.id == .r_paren) break;

                if (param_token.id == .identifier) {
                    const param_name = ast.tokenSlice(param_token);
                    const param_type = getParamType(ast, i);

                    if (first_param) {
                        params = std.fmt.comptimePrint("{{\"{s}\": \"{s}\"", .{ param_name, param_type });
                        first_param = false;
                    } else {
                        params = std.fmt.comptimePrint("{s}, \"{s}\": \"{s}\"", .{ params, param_name, param_type });
                    }
                }
            }
        }
    }

    return std.fmt.comptimePrint("{s}}}", .{params});
}

fn getParamType(comptime ast: std.zig.Ast, param_index: usize) []const u8 {
    var i = param_index;
    while (ast.tokens.len > i) : (i += 1) {
        const token = ast.tokens.get(i);
        switch (token.id) {
            .keyword_i32 => return "integer",
            .keyword_f32, .keyword_f64 => return "number",
            .keyword_bool => return "boolean",
            .keyword_anytype => return "any",
            else => continue,
        }
    }
    return "unknown";
}
