const std = @import("std");

/// Maps Zig types to JSON schema types
pub fn getTypeName(comptime T: type) []const u8 {
    return switch (@typeInfo(T)) {
        .Int => "integer",
        .Float => "number",
        .Bool => "boolean",
        .Pointer => "string",
        .Array => "array",
        .Struct => "object",
        .Optional => "nullable",
        .Enum => "enum",
        .Union => "union",
        .ErrorSet => "error",
        .Vector => "array",
        else => "unknown",
    };
}

/// Generates a JSON schema string for a function's parameters at compile time
pub fn generateFunctionSchema(comptime func: anytype) []const u8 {
    const type_info = @typeInfo(@TypeOf(func));
    if (type_info != .Fn) {
        @compileError("Provided type is not a function");
    }

    const params = type_info.Fn.args;
    var schema: []const u8 = "";

    for (params, 0..) |param, i| {
        const param_name = if (param.name) |name| name else "arg" ++ std.fmt.comptimePrint("{d}", .{i});
        const param_type = param.ty;
        const type_name = getTypeName(param_type);

        if (i == 0) {
            schema = std.fmt.comptimePrint("{{\"{s}\": \"{s}\"", .{ param_name, type_name });
        } else {
            schema = std.fmt.comptimePrint("{s}, \"{s}\": \"{s}\"", .{ schema, param_name, type_name });
        }
    }

    return std.fmt.comptimePrint("{s}}}", .{schema});
}

/// Creates a WASM-compatible string pointer from a comptime string
pub fn createWasmString(comptime str: []const u8) [*:0]const u8 {
    // Return a pointer to the first character of the string.
    return &str[0];
}

/// Automatically generates a describe function for all exported functions
pub fn createDescribeFunction(comptime exports: type) fn () [*:0]const u8 {
    var schema_parts: []const u8 = "";
    var first = true;

    inline for (std.meta.fields(exports)) |field| {
        const value = @field(exports, field.name);
        if (@TypeOf(value) == fn () [*:0]const u8) {
            const func_schema = value();
            if (first) {
                schema_parts = std.fmt.comptimePrint("{{\"{s}\": {s}", .{ field.name, func_schema });
                first = false;
            } else {
                schema_parts = std.fmt.comptimePrint("{s}, \"{s}\": {s}", .{ schema_parts, field.name, func_schema });
            }
        }
    }

    const final_schema = std.fmt.comptimePrint("{s}}}", .{schema_parts});

    return struct {
        pub fn describe() [*:0]const u8 {
            return createWasmString(final_schema);
        }
    }.describe;
}

/// Helper macro to create a schema getter function for a given function
pub fn createSchemaGetter(comptime func: anytype, comptime _getter_name: []const u8) fn () [*:0]const u8 {
    // Suppress the unused parameter warning.
    _ = _getter_name;
    const schema = generateFunctionSchema(func);
    return struct {
        pub fn getter() [*:0]const u8 {
            return createWasmString(schema);
        }
    }.getter;
}
