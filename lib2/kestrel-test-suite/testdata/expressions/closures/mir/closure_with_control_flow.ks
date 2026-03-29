// test: diagnostics
// stdlib: false

module Test

func test() -> (lang.i64) -> lang.i64 {
    { (x) in
        if lang.i64_signed_gt(x, 0) {
            x
        } else {
            lang.i64_sub(0, x)
        }
    }
}
