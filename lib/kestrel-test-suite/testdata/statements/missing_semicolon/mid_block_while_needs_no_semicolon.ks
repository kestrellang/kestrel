// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 3\n

// F13 guard: statement-like expressions legitimately carry no `;` — and
// not only as a block's trailing expression. Mid-block, the parser's
// block-end lookahead does not fire, so a zero-width `;` is synthesised; it
// must be routed to `BlockItem::StatementExpr` and NOT reported as missing.
// `lang/` alone has ~1439 such sites, so a false positive here breaks the
// entire stdlib.

module Test

@main
func main() {
    var t = 0;
    while t < 3 { t = t + 1; }
    print("\(t)");
}
