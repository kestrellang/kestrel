// test: diagnostics
// stdlib: true

module Main

// NEGATIVE: an ordinary `for` body must not warn. The desugared shape
// (`let $iter` + `loop { match $iter.next() { .Some => .., .None => break } }`)
// must not be mistaken for dead code now that the analyzer walks into it.
func test() {
    var total: std.numeric.Int64 = 0;
    for i in std.core.Range[std.numeric.Int64](0, 5) {
        total = total + i;
    }
}
