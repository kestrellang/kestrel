// test: execution
// stdlib: false

// Regression for #126's while-let twin: a `while let` with several
// comma-chained `let` bindings must thread the bindings into the loop body.
// The old `loop { if !cond { break }; body }` shape routed the patterns through
// the boolean-AND path, which dropped the bindings and tripped OSSA
// verification ("used but never defined"). Shared lowering: the condition chain
// in kestrel-hir-lower (lower_condition_chain / desugar_while_let_chain).
module Main

enum Option[T] {
    case Some(T)
    case None
}

// Yields .Some(n+1) until n reaches 3, then .None — a bounded generator.
func step(n: lang.i64) -> Option[lang.i64] {
    if lang.i64_signed_lt(n, 3) {
        let r: Option[lang.i64] = .Some(lang.i64_add(n, 1));
        r
    } else {
        let r: Option[lang.i64] = .None;
        r
    }
}

func always10() -> Option[lang.i64] {
    let r: Option[lang.i64] = .Some(10);
    r
}

@main
func main() -> lang.i64 {
    var i = 0;
    var total = 0;
    // Both bindings used in the body; loop ends when `step` yields .None.
    while let .Some(a) = step(i), let .Some(b) = always10() {
        total = lang.i64_add(total, lang.i64_add(a, b));
        i = a;
    }
    // i: 0→1→2→3 (3 iterations). a = 1,2,3 ; b = 10 each.
    // total = (1+10) + (2+10) + (3+10) = 36.
    if lang.i64_ne(total, 36) { return 1; }
    if lang.i64_ne(i, 3) { return 2; }
    0
}
