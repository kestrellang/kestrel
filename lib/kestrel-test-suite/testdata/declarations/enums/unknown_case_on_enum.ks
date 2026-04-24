// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
    case Green
    case Blue
}

func test() -> Color {
    Color.Purple // ERROR: undefined name
}
