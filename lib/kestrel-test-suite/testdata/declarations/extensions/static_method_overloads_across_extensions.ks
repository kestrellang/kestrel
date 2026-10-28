// test: execution
// stdlib: true
// expect-exit: 0

// F10: static-member lookup truncated to the FIRST `extend` block that had a
// match, so splitting overloads across two extensions silently dropped every
// overload but the first — `Foo.make(b:)` failed with "no matching overload"
// while `Foo.make(a:)` compiled. Truncation at name resolution is
// unrecoverable: the set becomes `HirExpr::OverloadSet` verbatim and
// inference never re-widens a single `Def`. Splitting one `extend` block in
// two is a pure refactor and must not change what resolves.

module Test

import std.numeric.Int64

struct Foo { let n: Int64 }

extend Foo { static func make(a v: Int64) -> Foo { Foo(n: v) } }
extend Foo { static func make(b v: Int64) -> Foo { Foo(n: v + 100) } }

@main
func main() -> lang.i32 {
    if Foo.make(a: 1).n != 1 { return 10 }
    if Foo.make(b: 1).n != 101 { return 20 }
    0
}
