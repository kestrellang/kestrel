// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: `sharedMutRef()` is the interior-mutability
// primitive — mutable access to the payload through a *shared*, non-`mutating`
// handle. The handle below is `let`-bound and the call is not `mutating`, yet
// the write lands in the shared storage: an alias observes it and no
// copy-on-write fork happens (uniqueness is unchanged).
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    let box = RcBox[Int64](1);            // `let`: the handle itself never mutates
    let alias = box.clone();              // second handle onto the same storage

    box.sharedMutRef() = 42;              // write through the &mutating projection

    if box.getValue() != 42 { return 1; }
    if alias.getValue() != 42 { return 2; }   // the alias sees it — shared, not forked

    if box.isUnique() { return 3; }       // sharedMutRef must not split storage
    if box.isIdentical(to: alias) == false { return 4; }
    0
}
