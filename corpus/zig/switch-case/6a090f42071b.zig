pub fn foo(x: u8) u8 {
    return switch (x) {
        100 => 200,
        0 => 400,
        else => 255,
    };
}
