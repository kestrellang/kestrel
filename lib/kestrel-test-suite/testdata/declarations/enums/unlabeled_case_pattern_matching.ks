// test: diagnostics
// stdlib: false

module Test

enum Option[T] {
    case Some(T)
    case None
}

func unwrap(opt: Option[lang.i64]) -> lang.i64 {
    match opt {
        .Some(v) => v,
        .None => 0
    }
}
