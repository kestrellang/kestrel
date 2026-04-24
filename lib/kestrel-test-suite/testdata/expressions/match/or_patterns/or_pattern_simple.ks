// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
}

func test(c: Color) -> lang.str {
    match c {
        .Red or .Green => "warm-ish",
        .Blue => "cool"
    }
}
