// test: diagnostics
// stdlib: false

module Main

enum Option {
    case Some(value: lang.i64)
    case None
}

func main() -> lang.i64 {
    let unwrap: (Option) -> lang.i64 = { (opt: Option) in
        match opt {
            .Some(v) => v,
            .None => 0
        }
    };
    unwrap(Option.Some(value: 42))
}
