// test: diagnostics
// stdlib: true

module Test
func mixed_types() -> (lang.i64, lang.str, lang.i1) { (1, "hello", true) }
func of_arrays() -> ([lang.i64], [lang.i64]) { ([1, 2], [3, 4]) }
