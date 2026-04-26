// test: execution
// stdlib: false

// Multiple free type params introduced by the conformance RHS — each gets
// its own TypeParam entity on the extension and each is independently
// inferable at the call site.

module Test

protocol Pair[A, B] {
    func describe(a a: A, b b: B) -> lang.i64
}

struct Tag { public init() {} }

extend Tag: Pair[A, B] {
    public func describe(a a: A, b b: B) -> lang.i64 { 7 }
}

func main() -> lang.i64 {
    let t = Tag();
    let a: lang.i64 = 100;
    let b: lang.i64 = 200;
    let r = t.describe(a: a, b: b);
    if lang.i64_eq(r, 7) { 0 } else { 1 }
}
