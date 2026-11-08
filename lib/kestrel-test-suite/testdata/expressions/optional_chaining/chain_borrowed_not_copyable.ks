// test: diagnostics
// stdlib: true

module Test

struct Token: not Copyable {
    var id: Int64
}

struct Holder {
    var token: Token
}

func test() {
    let h: Holder? = .None;
    // the tail reads through a borrow and `Token` can't be copied or cloned
    let t = h?.token; // ERROR(E503)
}
