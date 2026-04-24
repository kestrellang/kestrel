// test: diagnostics
// stdlib: true

module Main

func test(opt: std.num.Int64?) -> lang.i64 {
    match opt {
        .Some(_) => 1,
        .None => 0
    }
}
