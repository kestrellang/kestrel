// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `lang/datetime` shape: a type's own `static var zero`, plus a protocol
// witness of the same signature in an extension that forwards to it. Name
// resolution's walk lets the type's own member win; the static lookup must
// too, instead of calling the pair ambiguous (E116).

protocol HasZero { static var zero: Self { get } }

struct Meters { var v: Int64 }

extend Meters {
    static var zero: Meters { Meters(v: 0) }
}

struct Span { var n: Int64
    static var zero: Span { Span(n: 0) }
}

extend Span: HasZero {
    public static var zero: Span { Span.zero }
}

@main
func main() -> Int64 {
    if Span.zero.n != 0 { return 1 }
    if Meters.zero.v != 0 { return 2 }
    0
}
