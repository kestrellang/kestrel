// test: diagnostics
// stdlib: false

module Main

func send(to recipient: lang.str) -> lang.str { recipient }

func test() -> lang.str {
    send(to: "Alice")
}
