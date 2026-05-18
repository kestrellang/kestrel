// test: diagnostics
// stdlib: false

module Test

indirect enum List[T] { // ERROR: indirect enums are not yet supported
    case Cons(T, List[T])
    case Nil
}

func head(list: List[lang.i64]) -> lang.i64 {
    match list {
        .Cons(h, _t) => h,
        .Nil => 0
    }
}
