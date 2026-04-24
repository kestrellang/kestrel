// test: diagnostics
// stdlib: true

module Main

func test() {
    let x: std.num.Int64 = 100;
    for x in std.core.Range[std.num.Int64](0, 5) {
        ()
    }
}
