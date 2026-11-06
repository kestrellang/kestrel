// test: diagnostics

// A view closure that no binding carries (called immediately) lives only for
// its statement, so its freeze ends there: the place can be consumed on a
// later line. A closure kept in a binding freezes the place for the
// binding's extent.

module Test

struct R: not Copyable {
    var v: Int64;
    deinit { }
}

func eat(consuming r: R) -> Int64 { r.v }

func temporary() -> Int64 {
    let r = R(v: 1);
    let n = { r.v }();
    eat(r) + n
}

func kept() -> Int64 {
    let r = R(v: 1);
    let f = { r.v };
    let a = eat(r); // ERROR: cannot consume 'r.v' while a closure capturing it is live
    a + f()
}
