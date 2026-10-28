// test: execution
// stdlib: true
// expect-exit: 0

// Mutation through a `Result` base writes the `.Ok` payload in place and
// skips `.Err`.

module Test

struct Counter {
    var n: Int64
}

@main
func main() -> lang.i64 {
    var ok: Result[Counter, Int64] = .Ok(Counter(n: 1));
    ok?.n += 1;
    if ok?.n != .Some(2) { return 1 }
    var err: Result[Counter, Int64] = .Err(3);
    err?.n += 1;
    if err?.n != .None { return 2 }
    0
}
