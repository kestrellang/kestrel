// test: execution
// stdlib: true
// expect-exit: 0
//
// The member peel reaches `Indirection.Target` — the tagged associated type —
// not a same-named `Target` from another conformance. `View` also conforms to
// `Aimed`, whose own `Target` is `Int64`; reading `view.balance` must peel to
// `Account`.

module Test

import std.memory.(RcBox)
import std.core.(Indirection)

protocol Aimed {
    type Target
    func aim() -> Target
}

struct Account {
    var balance: Int64
}

struct View {
    var inner: RcBox[Account]
}

extend View: Aimed {
    type Aimed.Target = Int64
    func aim() -> Int64 { 7 }
}

extend View: Indirection {
    type Indirection.Target = Account
    public var value: Account { ref { self.inner.value } }
}

@main
func main() -> Int64 {
    let view = View(inner: RcBox(Account(balance: 42)));
    if view.balance != 42 { return 1 }
    if view.aim() != 7 { return 2 }
    0
}
