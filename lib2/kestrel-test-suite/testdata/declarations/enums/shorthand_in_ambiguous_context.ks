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
    let x = .Red; // ERROR: implicit member '.Red' not found
}
