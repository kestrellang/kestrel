// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Resource: not Copyable {
    var fd: lang.i64
}

struct Wrap[T] {
    var inner: T
}

// Forming `Wrap[Resource]` (Wrap is Copyable-by-default, so T: Copyable) in a
// type annotation must be rejected at the formation site.
func mk(r: Resource) -> Wrap[Resource] { // ERROR: !: Copyable
    return Wrap(inner: r)
}
