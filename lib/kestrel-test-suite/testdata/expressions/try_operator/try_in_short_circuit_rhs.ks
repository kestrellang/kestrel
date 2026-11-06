// test: diagnostics
// stdlib: true

// `try` on the right of `or` / `and` is rejected (E012): that side runs
// inside the short-circuit closure, so its early return would leave the
// closure, not the function. `try` on the left is fine, and so is a closure
// written inside the right side (its own scope).

module Test

func fetch(x: Int64) -> Int64? { .Some(x) }
func missing() -> Int64? { .None }

func rightOr() -> Bool? {
    if (try fetch(1)) != 1 or (try fetch(2)) != 2 { // ERROR(E012): `try` can't appear on the right side of `or`
        return .Some(false);
    }
    .Some(true)
}

func rightAnd() -> Bool? {
    if (try fetch(1)) == 1 and (try missing()) == 2 { // ERROR(E012): `try` can't appear on the right side of `and`
        return .Some(true);
    }
    .Some(false)
}

func leftOnly() -> Bool? {
    let ok = (try fetch(1)) == 1 or false;
    .Some(ok)
}

func check(f: () -> Int64?) -> Bool { f() == .Some(3) }

func insideClosure() -> Bool {
    false or check({ .Some(try fetch(3)) })
}
