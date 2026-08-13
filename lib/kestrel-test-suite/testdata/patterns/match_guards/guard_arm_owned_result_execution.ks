// test: execution
// stdlib: true
// expect-exit: 0
//
// A guard arm's condition (`digit if digit.isAsciiDigit => ...`) is an @owned
// temporary that the branch only READS: neither the arm body nor the remaining
// patterns consume it, so it must be destroyed in BOTH successors of the guard
// branch. When it wasn't, it reached each arm's leaf live with no consumer and
// the arms' owned results (String) tripped OSSA verify at codegen —
// "@owned value ... is live at block exit but never consumed" — while
// `kestrel check` passed. Found rewriting `lang/quill-json/src/Parser.ks`.

module Test

func classify(c: Char) -> String {
    match c {
        'a' => "letter",
        digit if digit.isAsciiDigit => "digit",
        _ => "other"
    }
}

// Two guard arms in a row: the failure branch of the first guard must restore
// the scope to its entry depth so the second guard's condition threads cleanly.
func bucket(n: Int64) -> String {
    match n {
        x if x < 0 => "negative",
        x if x == 0 => "zero",
        x if x < 10 => "small",
        _ => "large"
    }
}

@main
func main() -> lang.i64 {
    if classify('a') != "letter" { return 1 }
    if classify('7') != "digit" { return 2 }
    if classify('!') != "other" { return 3 }
    if bucket(-5) != "negative" { return 4 }
    if bucket(0) != "zero" { return 5 }
    if bucket(3) != "small" { return 6 }
    if bucket(99) != "large" { return 7 }
    0
}
