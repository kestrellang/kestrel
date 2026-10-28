// test: execution
// stdlib: true
// expect-exit: 0

// Regression (#197): a `\(...)` hole whose expression is itself an interpolated
// string was emitted verbatim (raw source, quotes and all). The AST builder had
// two divergent lowering paths — `ExprString` routed through the working
// token-parsing path while `ExprInterpolatedString` (produced by reparsing an
// interpolation hole) fell into a dead structured-children handler whose
// raw-token fallback dumped the literal source. Both node kinds carry the same
// single raw String token, so they now share one lowering path
// (`lower_string_token`); nested `\(...)` holes are lowered recursively.

module Test

@main
func main() -> lang.i64 {
    let name = "world";
    let n = 42;

    // one level of nesting — the canonical #197 case.
    if "msg: \("got: \(name)")" != "msg: got: world" { return 1 }

    // triple nesting.
    if "L1 \("L2 \("L3 \(name)")")" != "L1 L2 L3 world" { return 2 }

    // nested hole carrying a formatted integer.
    if "v=\("inner \(n)")" != "v=inner 42" { return 3 }

    // format spec applied inside a nested hole.
    if "hex: \("\(n:x)")" != "hex: 2a" { return 4 }

    // multi-line literal with a nested hole.
    let m = """
    multi \("deep \(name)")
    """;
    if m != "multi deep world" { return 5 }

    // plain (single-level) interpolation still works.
    if "hi \(name) and \(n)" != "hi world and 42" { return 6 }

    0
}
