// test: diagnostics
// stdlib: false

module Main

func findFirst(limit: lang.i64) -> lang.i64 {
    var i = 0;
    while true {
        if lang.i64_signed_ge(i, limit) {
            break
        }
        i = lang.i64_add(i, 1);
    }
    i
}
