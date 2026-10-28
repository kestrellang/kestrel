// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#204): `try` error-propagation must MOVE the non-Copyable Err
// payload into the re-wrapped Result, not borrow-clone-then-drop it. Before the
// fix the implicit-member static call `.fromResidual(payload)` hardcoded
// ParamConvention::Borrow, so the payload was borrowed, cloned into `.Err`, and
// the original dropped — firing the payload's deinit TWICE (a double-free; with a
// heap String payload this was a use-after-free / SIGILL). The fix collects real
// per-param conventions at the call site and marks `FromResidual.fromResidual`'s
// param `consuming`. Both backends: the double-free hit both.
//
// `runErr` scopes the propagated Err so its single deinit is observable; the
// payload must deinit EXACTLY once (was 2 / crash before the fix).

module Test

import std.numeric.Int64

public var edeinit: Int64 = 0;

struct E2: not Copyable {
    var id: Int64
    deinit { edeinit = edeinit + 1; }
}

func make() -> Result[Int64, E2] {
    .Err(E2(id: 1))
}

func use() -> Result[Int64, E2] {
    let r = try make();   // propagates `.Err` — payload must move, not double-drop
    .Ok(r)
}

func runErr() -> Bool {
    let b = use();
    b.isErr()
}   // `b` (the propagated Err) drops here -> exactly one payload deinit

@main
func main() -> lang.i64 {
    let isErr = runErr();
    if not isErr { return 1 };        // must be Err
    if edeinit != 1 { return 2 };     // exactly one deinit (was 2 / crash before fix)
    0
}
