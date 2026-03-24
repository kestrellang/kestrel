// test: diagnostics
// stdlib: false

module Main

struct NotIterable {
    let value: std.num.Int
}

func test() {
    let x = NotIterable(value: 42);
    for item in x { // ERROR: Iterable
        ()
    }
}
