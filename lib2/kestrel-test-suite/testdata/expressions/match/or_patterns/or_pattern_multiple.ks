// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Orange
    case Yellow
    case Green
    case Blue
    case Purple
}

func test(c: Color) -> lang.str {
    match c {
        .Red or .Orange or .Yellow => "warm",
        .Green or .Blue or .Purple => "cool"
    }
}
