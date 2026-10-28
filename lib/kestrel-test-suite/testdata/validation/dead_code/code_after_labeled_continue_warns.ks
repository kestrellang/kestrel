// test: diagnostics
// stdlib: false
//
// G12: the `continue` half of `code_after_labeled_break_warns.ks`. A labeled
// `continue` restarts the loop it names, so the statement after it never runs.

module Main

func test() {
    outer: loop {
        continue outer;
        let x: lang.i64 = 1; // WARN: unreachable
    }
}
