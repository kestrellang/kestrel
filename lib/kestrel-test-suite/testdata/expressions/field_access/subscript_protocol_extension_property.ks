// test: execution
// stdlib: true
// expect-exit: 0

module Main

// Subscripting a computed property that comes from a protocol extension,
// written as one call (`a.box(2)`). The getter's `Self` must bind to the
// receiver; it used to reach the mangler unbound (ICE).

struct Box {
    let n: Int64
    public subscript(i: Int64) -> Int64 { self.n + i }
}

protocol HasN {
    func value() -> Int64
}

extend HasN {
    public var box: Box { Box(n: self.value()) }
}

struct A: HasN {
    func value() -> Int64 { 10 }
}

@main
func main() -> Int64 {
    let a = A();
    if a.box(2) != 12 { return 1 }
    // The stdlib case that found it: `bytes` on a StringSlice comes from `extend Str`.
    let slice = "port=80".trimmed();
    if slice.bytes(0) != 112 { return 2 }
    if slice.bytes(5..).toOwned() != "80" { return 3 }
    0
}
