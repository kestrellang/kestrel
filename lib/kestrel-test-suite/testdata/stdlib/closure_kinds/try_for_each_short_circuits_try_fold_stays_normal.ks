// test: execution
// stdlib: true
// expect-exit: 0

// `tryForEach` becomes a direct loop taking a `mutating` closure, so it
// short-circuits on the first `.Err` and returns it — without forcing
// `tryFold` to become `mutating` (docs/design/closures-stdlib-audit.md,
// "`mutating`: Eager Write-Back Callbacks" + the last row of its test matrix,
// and docs/design/closures.md — "mutating: write-back"). `tryFold` still
// accepts a plain `let`-bound normal closure, reusable across calls.
module Test

@main
func main() -> lang.i64 {
    // tryForEach stops at the first Err and hands it back unchanged.
    var visited: Int64 = 0;
    let stopped = [1, 2, 3, 4, 5].iter().tryForEach({ (x) in
        visited = visited + 1;
        if x == 3 {
            let err: Result[(), Int64] = .Err(x);
            err
        } else {
            .Ok(())
        }
    });
    if visited != 3 { return 1 }
    match stopped {
        .Ok(_) => { return 2 },
        .Err(e) => { if e != 3 { return 3 } }
    }

    // tryFold is a normal-closure API: a read-only `let` combiner is accepted
    // and the binding survives the call (normal closures are Copyable).
    let combine: (Int64, Int64) -> Result[Int64, Int64] = { (acc, x) in
        if x < 0 {
            let err: Result[Int64, Int64] = .Err(x);
            err
        } else {
            .Ok(acc + x)
        }
    };

    let sum = [1, 2, 3].iter().tryFold(from: 0, by: combine);
    match sum {
        .Ok(v) => { if v != 6 { return 4 } },
        .Err(_) => { return 5 }
    }

    let stoppedFold = [1, -2, 3].iter().tryFold(from: 0, by: combine);
    match stoppedFold {
        .Ok(_) => { return 6 },
        .Err(e2) => { if e2 != -2 { return 7 } }
    }

    0
}
