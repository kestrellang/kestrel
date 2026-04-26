// test: diagnostics
// stdlib: false

module Main

struct Handler[T] {
    let handle: (T) -> T
}

func test() -> lang.i64 {
    let h = Handler[lang.i64](handle: { lang.i64_mul(it, 2) });
    (h.handle)(21)
}
