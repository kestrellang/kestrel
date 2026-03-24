// test: diagnostics
// stdlib: false

module Main

struct Inner { let value: lang.i64 }
struct Middle { let inner: Inner }
struct Outer { let middle: Middle }
struct Top { let outer: Outer }

func getValue(t: Top) -> lang.i64 {
    t.outer.middle.inner.value
}
