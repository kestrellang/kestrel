// test: diagnostics
// stdlib: false

// A view closure capturing a local, passed to a function that wraps it in a
// struct and returns it. Stored fields cannot hold view closures
// (E626, docs/design/closure-fields.md), so `Reg` is rejected at its
// declaration and the escape through `register(f:)` cannot be written; with
// an `escaping` field the literal snapshots `k`.

module Test

struct Reg {
    var h: (lang.i64) -> lang.i64 // ERROR(E626)
}

// Legal on its own: the caller's frame outlives this call.
func register(f: (lang.i64) -> lang.i64) -> Reg { Reg(h: f) }

func build() -> Reg {
    let k: lang.i64 = 5;
    register({ lang.i64_add(it, k) })
}
