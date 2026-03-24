// test: diagnostics
// stdlib: true

module Main

func test() {
    for i in std.core.Range[std.num.Int64](0, 10) {
        break nonexistent // ERROR: undeclared label
    }
}
