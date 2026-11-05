// test: execution
// stdlib: true

// `try` on the right of `or` / `and` runs only when the left side doesn't
// decide the result, and still returns early from the function on `None`.
// Today the right-hand `try` fails to infer its residual type (E104
// ".fromResidual not found on _"), while the same `try` on the left works.

module Test

func fetch(x: Int64) -> Int64? { .Some(x) }
func missing() -> Int64? { .None }

func bothMatch() -> Bool? {
    if (try fetch(1)) != 1 or (try fetch(2)) != 2 {
        return .Some(false);
    }
    .Some(true)
}

func rightFails() -> Bool? {
    if (try fetch(1)) == 1 and (try missing()) == 2 {
        return .Some(true);
    }
    .Some(false)
}

@main
func main() -> lang.i64 {
    match bothMatch() {
        .Some(true) => {},
        _ => { return 1; }
    }
    if rightFails().isSome() { return 2; }
    0
}
