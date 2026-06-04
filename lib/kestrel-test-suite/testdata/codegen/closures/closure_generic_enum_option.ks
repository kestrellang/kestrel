// test: execution
// stdlib: true

module Test

enum OptionalTransform[T] {
    case Some(f: (T) -> T)
    case None
}

func apply_or_default[T](opt: OptionalTransform[T], x: T, default: T) -> T {
    match opt {
        .Some(f: f) => f(x),
        .None => default
    }
}

@main
func main() -> lang.i64 {
    let transform = OptionalTransform[std.numeric.Int64].Some(f: { (x) in x * 2 });
    let none = OptionalTransform[std.numeric.Int64].None;

    if apply_or_default[std.numeric.Int64](transform, 21, 0) != 42 { return 1 }
    if apply_or_default[std.numeric.Int64](none, 21, 42) != 42 { return 2 }
    0
}
