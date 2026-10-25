// test: execution
// stdlib: true
// expect-exit: 0

// The way to consume a place's payload: `take()` moves it out (leaving
// `.None`), and the chain runs on the temporary.

module Test

struct Token: not Copyable {
    var id: Int64
    consuming func spend() -> Int64 { self.id }
}

@main
func main() -> lang.i64 {
    var token: Token? = .Some(Token(id: 3));
    if token.take()?.spend() != .Some(3) { return 1 }
    if token.isSome() { return 2 }
    0
}
