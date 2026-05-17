// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
    case Yellow
}

func test(c: Color) -> lang.i64 {
    match c { // ERROR: exhaustive
        .Red or .Green => 1,
        .Blue => 2
    }
}
