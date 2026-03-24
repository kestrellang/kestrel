// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
}

enum TrafficLight {
    case Red
}

func test() {
    let x = .Red; // ERROR: cannot infer enum type for shorthand
}
