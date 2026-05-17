// test: execution
// stdlib: true

module Test

struct Inner {
    let a: std.numeric.Int64
    let b: std.numeric.Int64
}

struct Outer {
    let inner: Inner
}

func sum_inner(i: Inner) -> std.numeric.Int64 {
    i.a + i.b
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(a: 20, b: 22));
    // Access the intermediate inner struct and pass it
    if sum_inner(o.inner) != 42 { return 1 }
    0
}
