// test: diagnostics
// stdlib: false

module Test

// A two-protocol inheritance cycle written with *qualified* paths
// (`Test.B` rather than bare `B`), plus an associated-type reference that
// has to walk that cycle. The inherited-associated-type walk in name-res
// had no cycle guard, so this used to recurse forever and abort the whole
// compiler with a stack overflow. It must terminate and report the cycle.
protocol A: Test.B {} // ERROR(E459)
protocol B: Test.A {}

// `Missing` genuinely does not exist anywhere in the cycle, so a truthful
// "cannot find type" is the correct secondary diagnostic — the walk simply
// gives up instead of recursing.
func take[T](x: T.Missing) -> () where T: A {} // ERROR: cannot find type 'T.Missing' in this scope
