// test: execution
// stdlib: false
// expect-exit: 0

// `Self` inside a generic struct's own body resolves to `Struct[T]` —
// carrying the struct's own type parameters as args, not a bare entity.
// Mirrors the extension fix for `extend Struct[T]`.

module Main

struct Box[T] {
    public let value: T
    public init(value v: T) { self.value = v; }
    public static func make(value v: T) -> Self {
        Self(value: v)
    }
}

@main
func main() -> lang.i64 {
    let b = Box[lang.i64].make(value: 42);
    lang.i64_sub(b.value, 42)
}
