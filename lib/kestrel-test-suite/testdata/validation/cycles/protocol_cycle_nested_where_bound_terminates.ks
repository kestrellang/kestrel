// test: diagnostics
// stdlib: false

module Test

// Same cycle guard, reached through the *other* caller: a nested
// associated-type path (`T.Iter.Nope`) resolved via `resolve_assoc_type_nested`
// and a second where-clause bound (`T.Iter: Q`) rather than the direct
// `T: Protocol` bound.
protocol P: Q { // ERROR(E459)
    type Iter
}
protocol Q: P {
    type Elem
}

func walk[T](x: T.Iter.Nope) -> () where T: P, T.Iter: Q {} // ERROR: cannot find type 'T.Iter.Nope' in this scope
