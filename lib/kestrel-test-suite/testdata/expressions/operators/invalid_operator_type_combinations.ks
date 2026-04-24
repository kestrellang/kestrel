// test: diagnostics
// stdlib: false

module Main

func stringPlusInt() -> lang.i64 {
    "hello" + 5 // ERROR:
}

func logicalAndOnInt() -> lang.i64 {
    1 and 2 // ERROR:
}

func bitwiseAndOnBool() -> lang.i1 {
    true & false // ERROR:
}
