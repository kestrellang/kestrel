// test: execution
// stdlib: false
// expect-exit: 0

// `Self` inside `extend GenericType[T]` resolves to `GenericType[T]`,
// keeping the extension's type parameter bound. A `Self` method here can
// return a fresh instance for any T the extension is monomorphized at.

module Main

struct Pair[A, B] {
    public let first: A
    public let second: B
    public init(first f: A, second s: B) {
        self.first = f;
        self.second = s;
    }
}

extend Pair[A, B] {
    public static func make(a a: A, b b: B) -> Self {
        Self(first: a, second: b)
    }
}

@main
func main() -> lang.i64 {
    let p = Pair[lang.i64, lang.i64].make(a: 10, b: 32);
    lang.i64_sub(lang.i64_add(p.first, p.second), 42)
}
