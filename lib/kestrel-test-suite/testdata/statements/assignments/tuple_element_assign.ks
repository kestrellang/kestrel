// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#198): assigning to a tuple element on a local var
// (`t.0 = v`, `t.1 = expr`, nested `(t.0).1 = v`) used to fall into the
// `_ => {}` no-op arm of `lower_assign` — the RHS was computed then silently
// dropped with no store and no diagnostic. The `TupleIndex` place shapes now
// project a FieldAddr like a stored struct field.

module Test

import std.num.Int64

@main
func main() -> Int64 {
    var t = (1, 2);
    t.0 = 5;
    t.1 = t.1 + 100;
    if t.0 != 5 { return 1 };
    if t.1 != 102 { return 2 };

    // read-modify-write of one element must not disturb the other.
    t.0 = t.0 + t.1;
    if t.0 != 107 { return 3 };
    if t.1 != 102 { return 4 };

    // nested tuple element (parenthesized so `0.1` doesn't lex as a float).
    var n = ((1, 2), 3);
    (n.0).1 = 99;
    if (n.0).1 != 99 { return 5 };
    if (n.0).0 != 1 { return 6 };
    if n.1 != 3 { return 7 };
    0
}
