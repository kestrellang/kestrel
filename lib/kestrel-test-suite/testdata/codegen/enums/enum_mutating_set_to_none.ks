// test: execution
// stdlib: true

module Test

enum Option {
    case Some(value: std.numeric.Int64)
    case None
}

func is_some(opt: Option) -> std.core.Bool {
    match opt {
        .Some(value: _) => true,
        .None => false
    }
}

func clear(mutating opt: Option) {
    opt = Option.None;
}

@main
func main() -> lang.i64 {
    var opt = Option.Some(value: 42);
    if is_some(opt) == false { return 1 }

    clear(opt);
    if is_some(opt) { return 2 }

    0
}
