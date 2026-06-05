// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.numeric.Int64)
    case None
}

func double_in_place(mutating opt: Option) {
    opt = match opt {
        .Some(value: v) => Option.Some(value: v * 2),
        .None => Option.None
    };
}

func unwrap_or(opt: Option, default: std.numeric.Int64) -> std.numeric.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

@main
func main() -> lang.i64 {
    var opt = Option.Some(value: 21);
    double_in_place(opt);
    if unwrap_or(opt, 0) != 42 { return 1 }
    0
}
