// test: diagnostics
// stdlib: false

module Main

enum Color {
    case Red
    case Green
    case Blue
}

func isRed(c: Color) -> lang.i1 {
    match c {
        .Red => true,
        _ => false
    }
}
