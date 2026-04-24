// test: diagnostics
// stdlib: true

module Main

func test(opt: std.num.Int64?) -> lang.i64 {
    guard let .Some(_v) = opt else {
        return 0
    }
    1
}
