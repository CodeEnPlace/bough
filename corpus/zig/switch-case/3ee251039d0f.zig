pub fn foo(x: u8) u8 {
    return switch (x) {
        100 => std.math.maxInt(u64),
        300 => 400,
        else => 255,
    };
}
