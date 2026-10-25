// test: execution
// stdlib: true
// expect-exit: 0

// Other chain forms: calling an optional function, `if let` over a chain,
// a chain in a closure and with implicit `it`, a nested optional, a generic
// tail, and `!` inside a chain.

module Test

struct User {
    var name: Int64
    var nick: Int64?
}

struct Box[T] {
    var value: T
}

func unbox[T](o: Box[T]?) -> T? { o?.value }

@main
func main() -> lang.i64 {
    let f: ((Int64) -> Int64)? = .Some({ (x) in x + 1 });
    if f?.(2) != .Some(3) { return 1 }

    let u: User? = .Some(User(name: 7, nick: .Some(9)));
    if let n = u?.name {
        if n != 7 { return 2 }
    } else {
        return 3
    }

    let users: [User?] = [u, .None];
    let names = users.map({ it?.name });
    if names(0) != .Some(7) { return 4 }
    if names(1) != .None { return 5 }
    let viaPlaceholder = users.map(_?.name);
    if viaPlaceholder(0) != .Some(7) { return 9 }

    // `nick!` unwraps inside the chain
    if u?.nick! != .Some(9) { return 6 }

    // nested optional: the bind exposes the inner `Int64?`
    let nested: Int64?? = .Some(.Some(4));
    let inner = nested?.isSome();
    if inner != .Some(true) { return 7 }

    // generic tail promotes even when `T` is itself optional
    let b: Box[Int64?]? = .Some(Box(value: .None));
    let r: Int64?? = unbox(b);
    if r != .Some(.None) { return 8 }
    0
}
