// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> lang.i64 {
    match c {
        .Red => 1,
        .Red => 2, // WARN: unreachable
        .Green => 3,
        .Blue => 4
    }
}
