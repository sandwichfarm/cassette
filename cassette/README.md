# Cassette

Cassette is a Zig library for generating deterministic schemas and descriptions for WebAssembly modules. It provides tools for creating type-safe, compile-time generated schemas that can be used across different environments.

## Features

### Schema Generation
- Compile-time type mapping from Zig types to JSON schema types
- Automatic function parameter schema generation
- WASM-compatible string handling
- Support for basic Zig types (integers, floats, booleans, etc.)

### Auto Description
- AST-based automatic schema generation from source files
- Support for exported functions
- Parameter type inference
- Compile-time schema generation

## Usage

### Basic Schema Generation

```zig
const cassette = @import("cassette");

// Create a schema getter for a function
const schema = cassette.createSchemaGetter(myFunction, "my_function_schema");

// Generate a description for multiple functions
const exports = struct {
    pub const func1_schema = cassette.createSchemaGetter(func1, "func1_schema");
    pub const func2_schema = cassette.createSchemaGetter(func2, "func2_schema");
};

const describe = cassette.createDescribeFunction(exports);
```

### Auto Description Generation

```zig
const cassette = @import("cassette");

// Generate a description function from a source file
const describe = cassette.generateDescribeFunction("path/to/source.zig");
```

## Example

Here's a complete example using the plusone-wasm project:

```zig
// main.zig
export fn increment(value: i32) i32 {
    return value + 1;
}

export fn add(a: i32, b: i32) i32 {
    return a + b;
}

// auto_describe.zig
const cassette = @import("cassette");

const exports = struct {
    pub const increment_schema = cassette.createSchemaGetter(@import("main.zig").increment, "increment_schema");
    pub const add_schema = cassette.createSchemaGetter(@import("main.zig").add, "add_schema");
};

pub const describe = cassette.createDescribeFunction(exports);
```

## Building

Add cassette as a dependency in your `build.zig.zon`:

```zig
.{
    .name = "your-project",
    .version = "0.0.1",
    .dependencies = .{
        .cassette = .{
            .url = "https://github.com/yourusername/cassette/archive/main.tar.gz",
            .hash = "12200000000000000000000000000000000000000000000000000000000000000000",
        },
    },
}
```

Then in your `build.zig`:

```zig
const cassette_dep = b.dependency("cassette", .{
    .target = target,
    .optimize = optimize,
});

// Add to your module
obj.addImport("cassette", cassette_dep.artifact("cassette"));
```

## License

MIT License - see LICENSE file for details 