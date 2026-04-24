// test: diagnostics
// stdlib: false

module Main

func getString() -> lang.str { "hello" }

func test() -> lang.str {
    let s: lang.str = getString();
    s
}
