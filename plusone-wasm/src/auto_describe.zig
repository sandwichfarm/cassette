const cassette = @import("cassette");

const exports = struct {
    pub const increment_schema = cassette.createSchemaGetter(@import("main.zig").increment, "increment_schema");
    pub const add_schema = cassette.createSchemaGetter(@import("main.zig").add, "add_schema");
};

pub const describe = cassette.createDescribeFunction(exports);
