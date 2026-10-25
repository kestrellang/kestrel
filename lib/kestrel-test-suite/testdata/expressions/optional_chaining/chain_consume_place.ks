// test: diagnostics
// stdlib: true

module Test

struct Token: not Copyable {
    var id: Int64
    consuming func spend() -> Int64 { self.id }
}

func test() {
    var t: Token? = .Some(Token(id: 1));
    let a = t?.spend(); // ERROR: can't consume through `?.` on a place
}
