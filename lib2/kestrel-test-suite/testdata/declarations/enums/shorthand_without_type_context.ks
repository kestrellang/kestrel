// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
    case Green
    case Blue
}

func test() {
    let x = .Red; // ERROR: cannot infer enum type for shorthand
}
