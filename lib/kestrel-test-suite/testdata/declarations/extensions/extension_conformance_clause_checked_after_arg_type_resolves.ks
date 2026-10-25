// test: diagnostics
// stdlib: true

// An extension's `where T: Show` gate gives a different verdict for the same
// type depending on whether the argument's type was known when the member
// resolved. With an annotated `[NoShow]` argument the call is rejected; with an
// inline array literal it is accepted. `solve_conforms` / the member gate run
// while the literal's element type is still unresolved, `reify_tv` turns it
// into `HirTy::Error`, `type_satisfies` treats Error as "permit", and the
// constraint is marked solved and never re-checked. Found in the 2026-10
// architecture review at 9767d2dc. When the extension body actually uses the
// bound, the same shape fails post-monomorphization instead.
// EXPECTED TO FAIL until conformance checks return holds / fails / unknown and
// defer on unknown.
module Test

protocol Show {
    func show() -> String
}

struct Box[T] {
    var v: T;
}

extend Box[T] where T: Show {
    func describe() -> Int64 { 7 }
}

struct NoShow {
    var n: Int64;
}

func test() -> Int64 {
    let annotated: [NoShow] = [NoShow(n: 3)];
    let a = Box(v: annotated).describe(); // ERROR: no member 'describe' on type 'Box[Array[NoShow]]'
    let b = Box(v: [NoShow(n: 2)]).describe(); // ERROR: no member 'describe' on type 'Box[Array[NoShow]]'
    a + b
}
