// test: execution
// stdlib: true
// expect-exit: 0
//
// An indirect enum of Copyable payloads is Copyable by clone: a copy owns
// its own boxes, so mutating it leaves the original unchanged.

module Test

import std.numeric.(Int64)

indirect enum List {
    case Nil
    case Cons(Int64, List)

    mutating func bumpAll() {
        match self {
            .Nil => {},
            .Cons(&mutating head, &mutating tail) => {
                head = head + 100;
                tail.bumpAll();
            }
        }
    }

    func sum() -> Int64 {
        match self {
            .Nil => 0,
            .Cons(head, tail) => head + tail.sum()
        }
    }
}

@main
func main() -> Int64 {
    let original = List.Cons(1, .Cons(2, .Nil));
    var copy = original;
    copy.bumpAll();
    if original.sum() != 3 { return 1 }
    if copy.sum() != 203 { return 2 }
    0
}
