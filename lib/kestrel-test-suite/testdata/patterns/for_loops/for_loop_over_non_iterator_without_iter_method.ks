// test: diagnostics
// stdlib: true

module Main

struct NotIterable {
    let value: std.numeric.Int
}

func test() {
    let x = NotIterable(value: 42);
    for item in x { // ERROR: Iterable
        ()
    }
}
