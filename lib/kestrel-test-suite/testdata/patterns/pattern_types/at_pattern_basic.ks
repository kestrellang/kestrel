// test: diagnostics
// stdlib: false

module Main

indirect enum List[T] { // ERROR: indirect enums are not yet supported
    case Cons(head: T, tail: List[T])
    case Nil
}

func test(list: List[lang.i64]) -> lang.i64 {
    match list {
        node @ .Cons(head, _) => head,
        .Nil => 0
    }
}
