// test: execution
// stdlib: true
// expect-exit: 0
//
// Smoke test: all four string forms work end-to-end (lex → parse → AST →
// HIR → codegen → runtime).

module Test

@main
func main() -> lang.i64 {
    let n = 7;

    // Form 1: single-line cooked (escapes + interpolation)
    let s1 = "n = \(n), tab=\there";
    if s1 != "n = 7, tab=\there" { return 1 }

    // Form 2: multi-line cooked (escapes + interpolation + indent strip)
    let s2 = """
        n = \(n)
        tab\there
        """;
    if s2 != "n = 7\ntab\there" { return 2 }

    // Form 3: single-line raw (literal everything)
    let s3 = #"n = \(n), tab=\there"#;
    if s3 != "n = \\(n), tab=\\there" { return 3 }

    // Form 4: multi-line raw (literal everything, with indent strip)
    let s4 = ##"""
        n = \(n)
        tab\there
        contains "# inside
        """##;
    if s4 != "n = \\(n)\ntab\\there\ncontains \"# inside" { return 4 }

    0
}
