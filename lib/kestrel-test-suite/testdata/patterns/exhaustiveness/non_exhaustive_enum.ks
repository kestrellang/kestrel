// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> lang.i64 {
    match c { // ERROR: exhaustive
        .Red => 1,
        .Green => 2
    }
}
