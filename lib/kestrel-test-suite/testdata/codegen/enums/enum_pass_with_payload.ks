// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.num.Int64)
    case None
}

func double_if_some(opt: Option) -> std.num.Int64 {
    match opt {
        .Some(value: v) => v * 2,
        .None => 0
    }
}

func main() -> lang.i64 {
    let result = double_if_some(Option.Some(value: 21));
    if result != 42 { return 1 }
    0
}
