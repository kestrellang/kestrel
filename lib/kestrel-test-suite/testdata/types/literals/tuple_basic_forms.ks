// test: diagnostics
// stdlib: false

module Test
func single() -> (lang.i64,) { (1,) }
func two_elements() -> (lang.i64, lang.i64) { (1, 2) }
func multiple() -> (lang.i64, lang.i64, lang.i64) { (1, 2, 3) }
