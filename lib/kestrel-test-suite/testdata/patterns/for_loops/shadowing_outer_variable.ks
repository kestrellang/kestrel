// test: diagnostics
// stdlib: true

module Main

func test() {
    let x: std.numeric.Int64 = 100;
    for x in std.core.Range[std.numeric.Int64](0, 5) {
        ()
    }
}
