pub fn foo(x: u8) u8 {
    return switch (x) {
        100 => 200,
        300 => std.math.maxInt(u64),
        else => 255,
    };
}
