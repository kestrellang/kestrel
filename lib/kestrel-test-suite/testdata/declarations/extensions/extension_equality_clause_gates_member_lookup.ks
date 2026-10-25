// test: diagnostics
// stdlib: true

// An equality where-clause on a concrete-type extension does not gate member
// lookup: `extension_bounds_hold_impl` (type-infer conformance.rs) skips
// equality clauses as "out of scope, treat satisfied". So `plusOne()` resolves
// on `Box[String]` and the body's `self.v + 1` runs on a String — a silent
// miscompile. Found in the 2026-10 architecture review at 9767d2dc: the
// program printed a pointer value plus one (e.g. 94022575196305). No corpus
// test reached an extension equality clause at a call site before this one.
// EXPECTED TO FAIL until equality clauses are evaluated wherever extension
// bounds are.
module Test

struct Box[T] {
    var v: T;
}

extend Box[T] where T = Int64 {
    func plusOne() -> Int64 { self.v + 1 }
}

func test() -> Int64 {
    let ok = Box(v: 41).plusOne();
    let bad = Box(v: "str").plusOne(); // ERROR: no member 'plusOne' on type 'Box[String]'
    ok + bad
}
