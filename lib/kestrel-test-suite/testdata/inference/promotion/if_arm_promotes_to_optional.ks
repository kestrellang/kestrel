// test: execution
// stdlib: true
// expect-exit: 0

// bidi ruling D7(i): `FromValue` promotes at every check leaf. An `if` arm
// checked against `Int64?` promotes a bare `5` the same way an annotated `let`
// does. Before P3 the arms were equated (`Equal`), not coerced, so the `5`
// arm was rejected (".None not found on Error").

module Test

func opt(c: Bool) -> Int64? {
    if c { 5 } else { .None }
}

@main
func main() -> lang.i64 {
    let x: Int64? = if true { 5 } else { .None };
    if x != .Some(5) { return 1; }
    if opt(false) != .None { return 2; }
    if opt(true) != .Some(5) { return 3; }
    0
}
