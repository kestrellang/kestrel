// test: execution
// stdlib: true
// expect-exit: 0
//
// KNOWN ISSUE: dropping a long chain recurses once per node (each box's
// deinit drops the next node), so 100k nodes overflow the stack. This test
// fails until drop of indirect enums is made iterative.

module Test

import std.numeric.(Int64)

indirect enum List: not Copyable {
    case Nil
    case Cons(Int64, List)
}

@main
func main() -> Int64 {
    var list = List.Nil;
    var i = 0;
    while i < 100000 {
        list = .Cons(i, list);
        i = i + 1;
    }
    0
}
