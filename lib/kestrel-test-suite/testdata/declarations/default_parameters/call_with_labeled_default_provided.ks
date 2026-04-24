// test: diagnostics
// stdlib: false

module Main

func send(to recipient: lang.str = "default@example.com") { }

func test() {
    send(to: "alice@example.com")
}
