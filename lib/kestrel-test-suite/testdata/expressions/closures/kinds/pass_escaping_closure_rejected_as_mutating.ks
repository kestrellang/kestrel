// test: diagnostics
// stdlib: false

// Passing table, `escaping` row — the single rejected cell. An `escaping`
// closure passes as normal, `consuming`, or `escaping`, but NOT as `mutating`:
// its calls are shared (aliases exist), not exclusive.
// See docs/design/closures.md — "Passing: What Fits Where".
module Test

func takesMutating(mutating f: mutating () -> lang.i64) -> lang.i64 { f() }

func escapingToMutating(x: lang.i64) -> lang.i64 {
    var e: escaping () -> lang.i64 = { x };
    takesMutating(e) // ERROR(E624)
}
