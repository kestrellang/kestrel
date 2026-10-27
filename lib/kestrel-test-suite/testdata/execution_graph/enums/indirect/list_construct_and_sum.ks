// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: [1, 2, 3]
//
// An `indirect enum` list: construction, a borrowed recursive walk, and
// printing it. Only `Cons` (the recursive case) is boxed.

module Test

import std.numeric.(Int64)
import std.text.(String)
import std.io.(println)

indirect enum List {
    case Nil
    case Cons(Int64, List)

    func sum() -> Int64 {
        match self {
            .Nil => 0,
            .Cons(head, tail) => head + tail.sum()
        }
    }

    func describeTail() -> String {
        match self {
            .Nil => "]",
            .Cons(head, tail) => ", \(head)" + tail.describeTail()
        }
    }

    func describe() -> String {
        match self {
            .Nil => "[]",
            .Cons(head, tail) => "[\(head)" + tail.describeTail()
        }
    }
}

@main
func main() -> Int64 {
    let list = List.Cons(1, .Cons(2, .Cons(3, .Nil)));
    println(list.describe());
    if list.sum() != 6 { return 1 }
    0
}
