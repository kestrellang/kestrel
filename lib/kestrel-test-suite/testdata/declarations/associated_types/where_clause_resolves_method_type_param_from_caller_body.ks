// test: diagnostics
// stdlib: false
//
// Regression test: a method whose where clause references the method's own
// type parameter (or the enclosing type's) must still resolve when the
// inference context is a *different* body that calls the method. Previously
// `WorldResolver::where_clauses` resolved names in the current inference
// owner's scope instead of the method's, so names like `T` or `U` appeared
// out-of-scope when the method was called from another function.

module Test

protocol Iterator {
    type Item;
    func next() -> Item
}

struct IntIter {
    var value: lang.i64
}

extend IntIter: Iterator {
    type Item = lang.i64
    func next() -> Item { self.value }
}

// where-clause RHS `U` is a method-level type param.
func collect[T, U](iter: T) -> U where T: Iterator, T.Item = U {
    iter.next()
}

// Call site is a separate body, so `self.owner` in the resolver differs from
// `collect`. The where clause must still resolve `U` via `collect`'s scope.
func main() -> lang.i64 {
    let iter = IntIter(value: 42);
    collect(iter)
}
