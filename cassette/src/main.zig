pub const schema = @import("schema.zig");
pub const auto_describe = @import("auto_describe.zig");

// Re-export all schema functionality
pub const getTypeName = schema.getTypeName;
pub const generateFunctionSchema = schema.generateFunctionSchema;
pub const createWasmString = schema.createWasmString;
pub const createDescribeFunction = schema.createDescribeFunction;
pub const createSchemaGetter = schema.createSchemaGetter;

// Re-export auto_describe functionality
pub const generateDescribeFunction = auto_describe.generateDescribeFunction;
