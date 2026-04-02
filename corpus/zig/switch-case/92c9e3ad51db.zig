pub fn foo(x: u8) u8 {
    return switch (x) {
        std.math.maxInt(u64) => 200,
        300 => 400,
        else => 255,
    };
}
