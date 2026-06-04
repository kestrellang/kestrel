// test: execution
// stdlib: true

// Regression: a NON-generic type's throwing init / Optional-returning factory
// had its call-site type_args inferred from the *wrapper result type*
// (`Result[Inner, Err]` -> [Inner, Err]; `Optional[Outer]` -> [Outer]) even
// though the callee has zero type-params. Mono collection's arity check
// (`type_args.len() != callee.type_params.len()`) then skipped the instance,
// so the callee survived monomorphization as an unresolved `Callee::Direct`
// and post-mono verify ICE'd ("Callee::Direct not resolved to Callee::Resolved").
//
// The over-provision came from `infer_parent_type_args`'s ECS-fallback branch,
// which is taken only when the callee init is not yet in `module.functions` —
// i.e. it depends on lowering ORDER. Declaring the caller (Outer) *before* the
// callee (Inner) is what surfaces it, so the order below is load-bearing.
// (Mirrors datetime's ZonedDateTime.init -> DateTime(...) and TimeZone.find.)
module Test

enum Err { case Bad }

struct Outer {
    var inner: Inner
    var k: std.numeric.Int64

    // Throwing-init-in-match calling another type's throwing init.
    init(n n: std.numeric.Int64, k k: std.numeric.Int64) throws Err {
        let i = match Inner(n: n) {
            .Ok(v) => v,
            .Err(e) => throw e
        };
        self.inner = i;
        self.k = k;
    }

    // Optional-returning factory (over-provisions [Outer]).
    static func find(n n: std.numeric.Int64) -> std.result.Optional[Outer] {
        match Outer(n: n, k: 0) {
            .Ok(o) => .Some(o),
            .Err(_) => .None
        }
    }
}

struct Inner {
    var n: std.numeric.Int64
    init(n n: std.numeric.Int64) throws Err {
        if n < 0 { throw Err.Bad; }
        self.n = n;
    }
}

func main() -> lang.i64 {
    match Outer.find(n: 5) {
        some o => { if o.inner.n != 5 { return 1; } },
        null => return 2
    };
    match Outer.find(n: -1) {
        some _ => return 3,
        null => {}
    };
    0
}
