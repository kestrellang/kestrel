// test: diagnostics
// stdlib: false

module Main

func greet(name: lang.str = "World") -> lang.str {
    name
}

func test() -> lang.str {
    greet("Alice")
}
