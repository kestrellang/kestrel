// test: diagnostics
// stdlib: false

module Test
func trailing() -> (lang.i64, lang.i64, lang.i64) { (1, 2, 3,) }
func nested() -> ((lang.i64, lang.i64), (lang.i64, lang.i64)) { ((1, 2), (3, 4)) }
