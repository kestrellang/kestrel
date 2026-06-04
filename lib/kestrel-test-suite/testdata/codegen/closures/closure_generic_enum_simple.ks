// test: execution
// stdlib: true

module Test

enum MaybeAction[T] {
    case Action(f: (T) -> T)
    case NoAction
}

func apply[T](m: MaybeAction[T], x: T) -> T {
    match m {
        .Action(f: f) => f(x),
        .NoAction => x
    }
}

@main
func main() -> lang.i64 {
    let action = MaybeAction[std.numeric.Int64].Action(f: { (x) in x + 20 });
    if apply[std.numeric.Int64](action, 22) != 42 { return 1 }
    0
}
