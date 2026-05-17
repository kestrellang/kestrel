// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.numeric.Int64)
    case None
}

func maybe_double(v: std.numeric.Int64, should_double: std.core.Bool) -> Option {
    if should_double {
        Option.Some(value: v * 2)
    } else {
        Option.None
    }
}

func unwrap_or(opt: Option, default: std.numeric.Int64) -> std.numeric.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let doubled = maybe_double(21, true);
    let none = maybe_double(21, false);

    if unwrap_or(doubled, 0) != 42 { return 1 }
    if unwrap_or(none, 99) != 99 { return 2 }

    0
}
