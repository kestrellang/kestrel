// test: diagnostics
// stdlib: false
//
// G9: a `break outer` inside a *nested* loop still exits the labeled outer
// loop, so the code after the outer loop is reachable and must NOT warn.
// The dead-code walk used to stop at the nested `loop` boundary, conclude the
// outer loop had no exit, call it infinite, and flag everything after it.

module Main

func test() -> lang.i64 {
    outer: loop {
        loop {
            break outer;
        }
    }
    let z: lang.i64 = 1;
    z
}
