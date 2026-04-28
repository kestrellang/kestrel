// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.numeric.Int64)
    case None
}

func unwrap_or(opt: Option, default: std.numeric.Int64) -> std.numeric.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    var opt = Option.None;
    if unwrap_or(opt, 99) != 99 { return 1 }

    opt = Option.Some(value: 42);
    if unwrap_or(opt, 99) != 42 { return 2 }

    opt = Option.Some(value: 100);
    if unwrap_or(opt, 99) != 100 { return 3 }

    0
}
