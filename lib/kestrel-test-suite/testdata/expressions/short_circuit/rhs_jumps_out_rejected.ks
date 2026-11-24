// test: diagnostics
// stdlib: true

// The right side of `and` / `or` / `??` runs inside the short-circuit
// closure, so a `return`, `throw`, `break` or `continue` there would leave
// that closure, not the function or loop it is written in. Before E012 covered
// them, `return` silently returned from the closure (the function kept going)
// and `break` / `continue` crashed MIR lowering. They are rejected now, like
// `try` (see try_operator/try_in_short_circuit_rhs.ks).

module Test

struct E {}

func returnInOr(x: Int64) -> Bool {
    let ok = x > 0 or return false; // ERROR(E012): `return` can't appear on the right side of `or`
    ok
}

func returnInAnd(x: Int64) -> Bool {
    let ok = x > 0 and return true; // ERROR(E012): `return` can't appear on the right side of `and`
    ok
}

func returnInCoalesce(x: Int64?) -> Int64 {
    let v = x ?? return 7; // ERROR(E012): `return` can't appear on the right side of `??`
    v
}

func tryInCoalesce(x: Int64?, y: Int64?) -> Int64? {
    .Some(x ?? (try y)) // ERROR(E012): `try` can't appear on the right side of `??`
}

func throwInOr(x: Bool) -> Bool throws E {
    .Ok(x or throw E()) // ERROR(E012): `throw` can't appear on the right side of `or`
}

func breakInOr() -> Int64 {
    var i = 0;
    while true {
        i = i + 1;
        let go = i < 3 or break; // ERROR(E012): `break` can't appear on the right side of `or`
    }
    i
}

func continueInOr() -> Int64 {
    var i = 0;
    var hits = 0;
    while i < 10 {
        i = i + 1;
        let even = i % 2 == 0 or continue; // ERROR(E012): `continue` can't appear on the right side of `or`
        if even { hits = hits + 1 }
    }
    hits
}

// A labeled `break` from a loop inside the right side, aimed at a loop
// outside it, still leaves the closure.
func labeledBreakOut() -> Int64 {
    var i = 0;
    outer: while i < 10 {
        i = i + 1;
        let stop = i > 5 or (if i > 2 { while true { break outer } ; false } else { false }); // ERROR(E012): `break` can't appear on the right side of `or`
    }
    i
}
