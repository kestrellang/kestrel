// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// An extension of a sub-protocol must see the parent protocol's static
// requirements through `Self`, with the parent's `Self` read as the
// sub-protocol's. `Self.make(...)` inside `extend Defaulted` used to be
// E132 "undefined name 'Self.make'" even though `Defaulted: Makeable`.

protocol Makeable {
    static func make(n: Int64) -> Self?
}

protocol Defaulted: Makeable {
    static func defaultValue() -> Int64
}

extend Defaulted {
    static func makeDefault() -> Self? { Self.make(Self.defaultValue()) }
}

struct Box: Makeable, Defaulted {
    var value: Int64

    static func make(n: Int64) -> Box? {
        if n < 0 { return null }
        Box(value: n)
    }

    static func defaultValue() -> Int64 { 7 }
}

@main
func main() -> lang.i64 {
    match Box.makeDefault() {
        .Some(b) => { if b.value != 7 { return 1 } },
        _ => { return 2 }
    }
    0
}
