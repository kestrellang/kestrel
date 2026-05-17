// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func maybeDouble(opt: Option[lang.i64]) -> lang.i64 {
    var result: lang.i64 = 0;
    if let .Some(v) = opt {
        result = lang.i64_mul(v, 2);
    }
    result
}
