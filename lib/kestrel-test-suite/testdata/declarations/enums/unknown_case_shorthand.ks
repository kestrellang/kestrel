// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
    case Green
    case Blue
}

func test() {
    let color: Color = .Purple; // ERROR: implicit member '.Purple' not found
}
