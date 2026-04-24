// test: diagnostics
// stdlib: false
module Test
func notAStruct() -> lang.i64 {
    42
}

func test() -> lang.i64 {
    notAStruct(x: 1) // ERROR: no matching overload
}
