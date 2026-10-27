// test: execution
// stdlib: true
// expect-exit: 0
//
// A generic `List[T]` instantiated with a Copyable, a Cloneable (`String`)
// and a move-only element type.

module Test

import std.numeric.(Int64)
import std.text.(String)

var drops: Int64 = 0;

struct Handle: not Copyable {
    var id: Int64
    deinit { drops = drops + 1; }
}

indirect enum List[T]: not Copyable where T: not Copyable {
    case Nil
    case Cons(head: T, tail: List[T])

    func count() -> Int64 {
        match self {
            .Nil => 0,
            .Cons(_, tail) => 1 + tail.count()
        }
    }
}

func strings() -> Int64 {
    let words: List[String] = .Cons(head: "a", tail: .Cons(head: "b", tail: .Nil));
    if words.count() != 2 { return 1 }
    match words {
        .Cons(head, _) => if head != "a" { return 2 },
        .Nil => return 3
    };
    0
}

func handles() -> Int64 {
    let hs: List[Handle] = .Cons(head: Handle(id: 1), tail: .Cons(head: Handle(id: 2), tail: .Nil));
    if hs.count() != 2 { return 1 }
    0
}

@main
func main() -> Int64 {
    let ints: List[Int64] = .Cons(head: 1, tail: .Cons(head: 2, tail: .Cons(head: 3, tail: .Nil)));
    if ints.count() != 3 { return 1 }
    let s = strings();
    if s != 0 { return 10 + s }
    let h = handles();
    if h != 0 { return 20 + h }
    if drops != 2 { return 30 + drops }
    0
}
