// test: diagnostics
// stdlib: false

module Main

func greet(name: lang.str) {}

func test() {
    greet(42); // ERROR
}
