// test: diagnostics
// stdlib: false

module Main

enum Option {
    case Some(value: lang.i64)
    case None
}

func unwrap(opt: Option) -> lang.i64 {
    match opt {
        .Some(v) => v,
        .None => 0
    }
}
