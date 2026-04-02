pub fn foo(x: u8) u8 {
    return switch (x) {
        100 => 200,
        1 => 400,
        else => 255,
    };
}
