// test: diagnostics
// stdlib: false

module Main

func double(x: lang.i64) -> lang.i64 { 42 }
func format(value: lang.i64, with prefix: lang.str) -> lang.str { prefix }

func test() -> lang.str {
    format(double(5), with: "Result: ")
}
