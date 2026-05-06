// test: diagnostics
// stdlib: true

// `tryFold[Acc, E]` has two method-level type parameters. When the combine
// closure only ever returns `.Ok(...)` and the binding has no annotation,
// nothing constrains `E` — inference emits `UnresolvedTypeParam` instead
// of silently defaulting to `Never`. See try_fold_adapter.ks for the
// correctly-annotated version.

module Test

func main() -> lang.i64 {
    let result = [1, 2, 3, 4].iter().tryFold(from: 0, combining: { (acc, x) in // ERROR: cannot infer type parameter
        .Ok(acc + x)
    });
    match result {
        .Ok(v) => { if v != 10 { return 1 } },
        .Err(_) => { return 2 }
    }
    0
}
