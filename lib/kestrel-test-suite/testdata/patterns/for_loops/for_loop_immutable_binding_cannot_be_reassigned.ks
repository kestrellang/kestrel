// test: diagnostics
// stdlib: true

module Main

func test() {
    for x in std.core.Range[std.num.Int64](0, 5) {
        x = x + 1 // ERROR: immutable
    }
}
