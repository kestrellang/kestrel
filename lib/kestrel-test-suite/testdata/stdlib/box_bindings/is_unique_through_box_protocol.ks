// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: true\ntrue\nfalse\ntrue\n
//
// `isUnique` is a `Box` requirement: a single-owner box is always unique,
// a shared one only until it is cloned. `Shared[T]` names the stdlib's
// shared box (`RcBox`) without spelling the mechanism.

module Test

import std.numeric.(Int64)
import std.memory.(Box, OwnedBox, Shared)
import std.streams.println

func unique[B](box: B) -> Bool where B: Box {
    box.isUnique
}

@main
func main() {
    let owned = OwnedBox(1);
    println(unique(owned));
    let shared: Shared[Int64] = Shared(2);
    println(unique(shared));
    let alias = shared.clone();
    println(unique(shared));
    let fresh: Shared[Int64] = Shared(3);
    println(unique(fresh));
}
