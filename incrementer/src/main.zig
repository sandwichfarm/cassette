export var memory: [65536]u8 align(65536) = undefined;

export fn increment(value: i32) i32 {
    return value + 1;
}

export fn add(a: i32, b: i32) i32 {
    return a + b;
}