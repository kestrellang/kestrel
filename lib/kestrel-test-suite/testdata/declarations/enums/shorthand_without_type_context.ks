// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
    case Green
    case Blue
}

func test() {
    let x = .Red; // ERROR: implicit member '.Red' not found
}
