// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
}

func test(c: Color) -> lang.i64 {
    match c {
        .Red => 1,
        .Blue => 2 // ERROR: Blue
    }
}
