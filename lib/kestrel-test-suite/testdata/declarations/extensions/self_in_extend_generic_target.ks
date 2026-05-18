// test: execution
// stdlib: false
// expect-exit: 0

// `Self` inside `extend GenericType[args]` resolves to the *parameterized*
// target — `Self` here means `Box[lang.i64]`, not bare `Box`. Without this,
// `-> Self` lowered to unparameterized `Box` and failed to unify with
// `Box[lang.i64]` at call sites ("expected Box got Box[i64]").

module Main

struct Box[T] {
    public let value: T
    public init(value v: T) { self.value = v; }
}

extend Box[lang.i64] {
    public static func zero() -> Self {
        Self(value: 0)
    }
    public static func of(v v: lang.i64) -> Self {
        Self(value: v)
    }
}

func main() -> lang.i64 {
    let z = Box[lang.i64].zero();
    let n = Box[lang.i64].of(v: 42);
    lang.i64_sub(n.value, lang.i64_add(z.value, 42))
}
