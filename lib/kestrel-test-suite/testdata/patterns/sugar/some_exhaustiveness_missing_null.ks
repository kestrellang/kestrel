// test: diagnostics
// stdlib: true

module Test

func test(opt: std.result.Optional[std.numeric.Int64]) -> std.numeric.Int64 {
    match opt { // ERROR: non-exhaustive match
        some x => x
    }
}
