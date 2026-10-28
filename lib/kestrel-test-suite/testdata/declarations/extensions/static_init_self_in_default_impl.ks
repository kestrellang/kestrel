// test: execution
// stdlib: true
// expect-exit: 0
//
// #146 (init facet): a STATIC protocol-extension default method that calls the
// `init()` requirement via `Self()` and/or returns `Self` must propagate
// self_type to its monomorphized body. A static default has no receiver param,
// so the witness resolver's "needs Self" detection has to also recognize Self
// in the return type — otherwise `Self` (TypeParam(protocol)) leaks
// unsubstituted to the mangler ("TypeParam(...) reached the mangler").

module Test

protocol Springy {
    init()
    func v() -> Int64
}

extend Springy {
    public static func makeFresh() -> Self { Self() }
}

struct S: Springy {
    var n: Int64;
    init() { self.n = 5; }
    func v() -> Int64 { self.n }
}

struct T: Springy {
    var n: Int64;
    init() { self.n = 9; }
    func v() -> Int64 { self.n }
}

// also reach it through a type parameter
func freshOf[X]() -> Int64 where X: Springy { X.makeFresh().v() }

@main
func main() -> lang.i32 {
    if S.makeFresh().v() != 5 { return 1 } // concrete receiver
    if T.makeFresh().v() != 9 { return 2 } // distinct conformer
    if freshOf[S]() != 5 { return 3 }      // via type parameter
    0
}
