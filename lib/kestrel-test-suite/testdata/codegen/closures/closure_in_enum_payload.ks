// test: execution
// stdlib: true

module Test

enum Action {
    case Transform(f: (std.num.Int64) -> std.num.Int64)
    case NoOp
}

func apply_action(a: Action, x: std.num.Int64) -> std.num.Int64 {
    match a {
        .Transform(f: f) => f(x),
        .NoOp => x
    }
}

func main() -> lang.i64 {
    let action = Action.Transform(f: { (x) in x * 2 });
    if apply_action(action, 21) != 42 { return 1 }
    0
}
