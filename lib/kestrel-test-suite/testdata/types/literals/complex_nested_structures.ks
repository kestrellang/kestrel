// test: diagnostics
// stdlib: true

module Test
func array_of_tuples() -> [(lang.i64, lang.i64)] { [(1, 2), (3, 4)] }
func deeply_nested() -> [[(lang.i64,)]] { [[(1,)]] }
