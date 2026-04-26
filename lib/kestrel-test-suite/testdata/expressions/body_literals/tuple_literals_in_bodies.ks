// test: diagnostics
// stdlib: false

module Main

func getSimpleTuple() -> (lang.i64, lang.str) { (42, "hello") }
func getNestedTuple() -> ((lang.i64, lang.i64), lang.str) { ((1, 2), "point") }
func getGroupedLiteral() -> lang.i64 { (42) }
