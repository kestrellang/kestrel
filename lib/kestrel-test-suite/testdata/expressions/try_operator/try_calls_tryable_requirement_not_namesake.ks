// test: execution
// stdlib: true
// expect-exit: 42
//
// `try v` splits `v` through the `Tryable.tryExtract` requirement. `Maybe`
// also witnesses `Peel.tryExtract()` (same name and labels, declared first),
// which `try` must never call.

module Test

protocol Peel {
    consuming func tryExtract() -> Int64
}

enum Maybe: Tryable, Peel {
    case Has(Int64)
    case Nothing
    type Output = Int64
    type Residual = ()
    consuming func tryExtract() -> Int64 { 7 }
    consuming func tryExtract() -> ControlFlow[Int64, ()] {
        match self {
            .Has(v) => .Continue(v),
            .Nothing => .Break(()),
        }
    }
}

func unwrap(m: Maybe) -> Int64? {
    let v = try m;
    .Some(v)
}

@main
func main() -> Int64 {
    match unwrap(Maybe.Has(42)) {
        .Some(v) => v,
        .None => 1,
    }
}
