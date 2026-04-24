// test: diagnostics
// stdlib: false

module Main

func format(value: lang.i64, with prefix: lang.str) -> lang.str { prefix }

func test() -> lang.str {
    format(42, "Result: ") // ERROR: no matching overload
}
