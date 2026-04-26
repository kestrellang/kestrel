// test: diagnostics
// stdlib: false

module Main

func greet(with name: lang.str) -> lang.str { name }

func test() -> lang.str {
    greet(using: "world") // ERROR: no matching overload
}
