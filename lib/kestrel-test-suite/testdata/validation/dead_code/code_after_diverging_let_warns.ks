// test: diagnostics
// stdlib: true
//
// G12: a `let` whose *initializer* diverges never binds anything, so the rest
// of the block is unreachable. Only `exhaustive_return`'s copy of the
// divergence rule ever looked at `HirStmt::Let`; the other five (dead code
// among them) matched `HirStmt::Expr` and nothing else, so this shape was
// silently reachable. The shared `control_flow::stmt_diverges` handles it once.

module Main

func boom() -> ! {
    fatalError("boom");
}

func test() {
    let x: Int64 = boom();
    let y: Int64 = x; // WARN: unreachable
}
