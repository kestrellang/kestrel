// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(T)
    case None
}

func find(limit: lang.i64) -> Option[lang.i64] {
    var x: lang.i64 = 0;

    while true {
        if lang.i64_signed_gt(x, limit) {
            return .Some(x)
        }
        x = lang.i64_add(x, 1);
    }

    // This is unreachable code (after an infinite loop),
    // but should not cause a parse/binding error
    .None
}
