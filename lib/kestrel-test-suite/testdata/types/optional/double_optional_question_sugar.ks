// test: execution
// stdlib: true
// expect-exit: 0

// #191: `T??` in type position is the double-optional sugar
// (Optional[Optional[T]]). The lexer produces a single `??` token (the
// nil-coalescing operator in expression position), so the type parser must
// treat it as two stacked `?`. The spelled-out form already worked.
module Test

import std.numeric.Int64

@main
func main() -> lang.i64 {
    let o: Optional[Optional[Int64]] = .Some(.Some(1));
    let p: Int64?? = .Some(.Some(2));

    let a = match o { .Some(.Some(x)) => x, _ => 0 };
    let b = match p { .Some(.Some(x)) => x, _ => 0 };
    if a != 1 { return 1; }
    if b != 2 { return 2; }

    // Outer .Some wrapping an inner .None is distinct from the outer .None.
    let q: Int64?? = .Some(null);
    let c = match q { .Some(.Some(_)) => 10, .Some(.None) => 20, .None => 30 };
    if c != 20 { return 3; }

    0
}
