// test: execution
// stdlib: true
// expect-exit: 0
//
// An `@` pattern binds the whole value of a boxed case alongside its
// payload, and generic methods work on an indirect enum.

module Test

import std.numeric.(Int64)

indirect enum List[T] {
    case Nil
    case Cons(head: T, tail: List[T])

    func length() -> Int64 {
        match self {
            .Nil => 0,
            .Cons(_, tail) => 1 + tail.length()
        }
    }

    static func empty() -> List[T] { .Nil }
}

func headPlusLength(list: List[Int64]) -> Int64 {
    match list {
        node @ .Cons(head, _) => head + node.length(),
        .Nil => 0
    }
}

@main
func main() -> Int64 {
    let list: List[Int64] = .Cons(head: 10, tail: .Cons(head: 20, tail: List[Int64].empty()));
    if headPlusLength(list) != 12 { return 1 }
    0
}
