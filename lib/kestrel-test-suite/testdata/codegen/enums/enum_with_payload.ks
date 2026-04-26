// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func unwrap_or(opt: Option, default: std.num.Int64) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v,
        .None => default
    }
}

func main() -> lang.i64 {
    let some = Option.Some(value: 42);
    if unwrap_or(some, 0) != 42 { return 1 }
    0
}
