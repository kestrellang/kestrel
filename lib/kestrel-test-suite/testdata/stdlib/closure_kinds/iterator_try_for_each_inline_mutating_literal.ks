// test: execution
// stdlib: true
// expect-exit: 0

// Pins `Iterator.tryForEach` as a `mutating`-closure API (docs/design/closures.md,
// "mutating: write-back"; closures-stdlib-audit.md lists it among the five eager
// side-effect APIs). An inline literal writes back to captured frame vars while
// still returning the `Result` that drives short-circuiting.
module Test

@main
func main() -> lang.i64 {
    // All-Ok run: every element is visited and the write-back lands.
    var total: Int64 = 0;
    let allOk = [1, 2, 3].iter().tryForEach({ (x) in
        total = total + x;
        let ok: Result[(), Int64] = .Ok(());
        ok
    });
    if total != 6 { return 1 }
    match allOk {
        .Ok(_) => {},
        .Err(_) => { return 2 }
    }

    // Writes made before the failing element are still observable afterwards.
    var visited: Int64 = 0;
    let failed = [1, 2, 3, 4].iter().tryForEach({ (x) in
        visited = visited + 1;
        if x == 3 {
            let err: Result[(), Int64] = .Err(x);
            err
        } else {
            .Ok(())
        }
    });
    if visited != 3 { return 3 }
    match failed {
        .Ok(_) => { return 4 },
        .Err(e) => { if e != 3 { return 5 } }
    }

    0
}
