// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

// Regression (#173): a zero-param closure must NOT be falsely rejected with
// E600 merely because a *sibling* closure in the same function uses `it`. The
// `it`-usage fact is per closure literal (tracked on its own TyVar in the
// solver), not per enclosing function body. `{ () in 42 }` contains no `it`;
// `{ it + 1 }` is a separate, single-param closure.
@main
func main() -> lang.i64 {
    let f: () -> Int64 = { () in 42 };
    let g: (Int64) -> Int64 = { it + 1 };
    if f() != 42 { return 1 }
    if g(1) != 2 { return 2 }
    0
}
