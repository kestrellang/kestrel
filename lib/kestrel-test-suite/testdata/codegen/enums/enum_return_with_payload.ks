// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.numeric.Int64)
    case None
}

func make_some(v: std.numeric.Int64) -> Option {
    Option.Some(value: v)
}

func main() -> lang.i64 {
    let opt = make_some(42);
    match opt {
        .Some(value: v) => {
            if v != 42 { return 1 }
            0
        },
        .None => 2
    }
}
