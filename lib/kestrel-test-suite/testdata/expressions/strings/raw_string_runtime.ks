// test: execution
// stdlib: true
// expect-exit: 0
//
// Regression: raw strings used to lower with an i8 length where i64 was
// expected, producing a backend verifier error like
//   "arg 2 (vN) has type i8, expected i64"
// at any callsite that consumed the literal. The pre-existing raw-string
// tests are all `diagnostics`-kind and only check parsing, so the bug
// went undetected.

module Test

func consume(s: String) -> Int64 {
    s.byteCount
}

@main
func main() -> lang.i64 {
    // Direct binding round-trips through codegen.
    let raw = #"hello"#;
    if raw.byteCount != 5 { return 1 }

    // Passing a raw literal as an argument was the original failure shape.
    if consume(#"hello"#) != 5 { return 2 }

    // Raw and regular literals must produce equal Strings.
    if raw != "hello" { return 3 }

    // Empty raw string has zero bytes (and isn't mistaken for a null pointer).
    let empty = #""#;
    if empty.byteCount != 0 { return 4 }
    if not empty.isEmpty { return 5 }

    0
}
