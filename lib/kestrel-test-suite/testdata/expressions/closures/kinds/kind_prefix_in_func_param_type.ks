// test: execution
// stdlib: true
// expect-exit: 0

// Pins the kind prefix in a function *parameter* type for all three keywords,
// each paired with the parameter convention the design mandates (`mutating`
// kind on a `mutating` param, `consuming` kind on a `consuming` param).
// Literals are built directly for the expected kind.
// See docs/design/closures.md — "Passing: What Fits Where".
module Test

import std.numeric.(Int64)

func callEscaping(f: escaping () -> Int64) -> Int64 { f() }

func callMutatingTwice(mutating f: mutating () -> ()) { f(); f(); }

func callConsuming(consuming f: consuming () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    let a = 4;
    if callEscaping({ a }) != 4 { return 1 }

    var total = 0;
    callMutatingTwice({ total = total + 3; });
    if total != 6 { return 2 }

    let b = 9;
    if callConsuming({ b }) != 9 { return 3 }

    0
}
