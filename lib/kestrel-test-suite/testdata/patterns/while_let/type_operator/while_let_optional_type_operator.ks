// test: diagnostics
// stdlib: true

module Main

func test() -> lang.i64 {
    var opt: std.numeric.Int64? = .Some(1);
    var seen: lang.i64 = 0;
    while let .Some(_v) = opt {
        seen = 1;
        opt = .None;
    }
    seen
}
