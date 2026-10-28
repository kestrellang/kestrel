// test: diagnostics
// stdlib: false
//
// G9: `break outer` from inside a nested loop exits the labeled outer loop, so
// the outer loop does NOT diverge and the function falls through to its end
// with no return — E001. The exhaustive-return walk used to stop at the nested
// `loop` boundary, see no break, call the outer loop infinite, and silently
// accept a function that returns garbage at runtime.

module Main

func test() -> lang.i64 {
    outer: loop {
        loop {
            break outer;
        }
    }
    let z: lang.i64 = 1;
} // ERROR: does not return a value on all code paths
