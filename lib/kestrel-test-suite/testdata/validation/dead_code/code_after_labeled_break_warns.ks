// test: diagnostics
// stdlib: false
//
// G12: `break outer;` inside the loop it names exits that loop, so anything
// after it in the same block is unreachable. Dead-code analysis used to treat
// every *labeled* break as conservatively non-diverging ("the label might
// target a non-enclosing loop") and stayed silent here. Divergence now comes
// from the shared `control_flow` predicate, where `break` is diverging
// unconditionally — the label only ever decides *which* loop it exits.

module Main

func test() {
    outer: loop {
        break outer;
        let x: lang.i64 = 1; // WARN: unreachable
    }
}
