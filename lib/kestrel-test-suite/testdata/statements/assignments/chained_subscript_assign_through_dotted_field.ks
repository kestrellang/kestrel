// test: execution
// stdlib: true
// expect-exit: 0

// Regression (#129): `x.field(i)(j) = v` — a chained subscript assignment
// whose intermediate receiver is a DOTTED stored-field access — silently
// mutated a discarded getter temp (memory corruption / bogus index traps).
// The dotted receiver surfaces as `HirExpr::MethodCall`, which
// `accessor_member_prelude` didn't recognise, so the get→slot→writeback
// machinery was skipped and `prepare_call_arg_for_expr` fell back to
// mut-borrowing an owned rvalue copy. The bare-`Call` chain
// (`cells(i)(j) = v` on a local) already worked.
//
// See lib/kestrel-mir-lower/src/body/mod.rs::accessor_member_prelude.

module Test

struct Grid {
    var cells: [[Int64]];

    subscript(r: Int64, c: Int64) -> Int64 {
        get { self.cells(r)(c) }
        set { self.cells(r)(c) = newValue; }
    }

    mutating func poke(r: Int64, c: Int64, v: Int64) {
        self.cells(r)(c) = v
    }
}

func pokeFree(mutating grid: Grid, r: Int64, c: Int64, v: Int64) {
    grid.cells(r)(c) = v
}

@main
func main() -> lang.i64 {
    // Inside a computed subscript setter (the original #129 repro).
    var g = Grid(cells: [[0, 0, 0], [0, 0, 0]]);
    g(1, 2) = 7;
    if g.cells(1)(2) != 7 { return 1 }

    // Inside a `mutating` method.
    g.poke(0, 1, 3);
    if g.cells(0)(1) != 3 { return 2 }

    // Through a `mutating` free-function parameter's field.
    pokeFree(g, 1, 0, 9);
    if g.cells(1)(0) != 9 { return 3 }

    // Regression pin: bare-`Call` chain on a local (already worked).
    var cells = [[0, 0], [0, 0]];
    cells(1)(1) = 5;
    if cells(1)(1) != 5 { return 4 }

    0
}
