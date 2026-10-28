// test: execution
// stdlib: true
// expect-exit: 0

// A normal (`let`-bound, view-capturing, Copyable) closure value passes into a
// `mutating` parameter through the normal -> `mutating` adapter — row 1 of the
// passing table in docs/design/closures.md. The adapter, not the `let` binding,
// supplies the mutable place, so the original binding stays usable afterwards.
module Test

public var log: Int64 = 0;

@main
func main() -> lang.i64 {
    // Normal kind: reads its captures only. Writing a module-level `var` is not
    // a capture, so this stays a legal normal body (no E603).
    let record: (Int64) -> () = { (x) in log = log * 10 + x };

    [1, 2, 3].iter().forEach(record);
    if log != 123 { return 1 }

    // Still usable: normal closures are Copyable and the mutating adapter does
    // not consume the source binding.
    [4].iter().forEach(record);
    if log != 1234 { return 2 }

    // The same value also feeds the other four mutating stdlib APIs.
    let ok: Result[Int64, Int64] = .Ok(5);
    let tapped = ok.inspect(record);
    if log != 12345 { return 3 }
    if tapped.unwrap() != 5 { return 4 }

    let present: Optional[Int64] = .Some(6);
    let inspected = present.inspect(record);
    if log != 123456 { return 5 }
    if inspected.unwrap() != 6 { return 6 }

    0
}
