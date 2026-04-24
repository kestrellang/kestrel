// test: diagnostics
// stdlib: false

module Main

func nested(x: lang.i64, y: lang.i64) -> lang.i64 {
    if lang.i64_signed_gt(x, 0) {
        if lang.i64_signed_gt(y, 0) {
            1
        } else {
            2
        }
    } else {
        if lang.i64_signed_gt(y, 0) {
            3
        } else {
            4
        }
    }
}
