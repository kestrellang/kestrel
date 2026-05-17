// test: diagnostics
// stdlib: false

module Test

type NestedTuple = ((lang.i64, lang.str), lang.i1)
type ComplexNesting = (lang.i64, (lang.str, (lang.i1, lang.f64)))
