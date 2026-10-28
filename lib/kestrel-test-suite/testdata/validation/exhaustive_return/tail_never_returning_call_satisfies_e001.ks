// test: diagnostics
// stdlib: true
//
// G12: a non-unit function whose last statement is a call to a `-> !` function
// never falls off its end, so E001 must not fire. The Never-typed leaf rule is
// the only thing that can see this — `boom()` is an ordinary `HirExpr::Call`
// with no syntactic tell. Before G12 there was zero coverage of `-> !`
// divergence for E001 (only two E003 files exercised it at all).

module Main

func boom() -> ! {
    fatalError("boom");
}

func pick(flag: Bool) -> Int64 {
    if flag {
        return 1;
    }
    boom();
}
