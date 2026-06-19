// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Res: not Copyable {
    var fd: lang.i64
}

struct Box[T] {
    var value: T
}

// Constructing a Copyable-by-default container (`Box[T]` carries an implicit
// `T: Copyable`) with a non-Copyable argument must be rejected at the
// construction site. The container's bound lives on `Box`, not on any function
// type param, so neither the call-site where-clause check nor the function's
// own signature sees it — the construction expression is the formation site.
// Regression: previously this escaped the frontend and tripped the post-mono
// containment ICE (Inv-3b / BUG-04 "Copyable mono-substitution gap").
func use(r: Res) {
    let b = Box(value: r); // ERROR: !: Copyable
}
