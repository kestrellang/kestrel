// test: execution
// stdlib: true

module Test

enum Action {
    case Transform(f: (std.numeric.Int64) -> std.numeric.Int64)
    case NoOp
}

func apply_action(a: Action, x: std.numeric.Int64) -> std.numeric.Int64 {
    match a {
        .Transform(f: f) => f(x),
        .NoOp => x
    }
}

@main
func main() -> lang.i64 {
    let action = Action.Transform(f: { (x) in x * 2 });
    if apply_action(action, 21) != 42 { return 1 }
    0
}
