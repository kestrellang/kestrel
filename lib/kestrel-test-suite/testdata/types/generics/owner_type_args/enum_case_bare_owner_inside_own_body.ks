// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Inside `Opt[T]`, bare `Opt.Some(v)` is `Opt[_]` (inferred from `v`), and
// `Self.None` is `Opt[T]`.

enum Opt[T] {
    case Some(T)
    case None

    static func wrap(v: T) -> Opt[T] { Opt.Some(v) }
    static func empty() -> Opt[T] { Self.None }
}

@main
func main() -> Int64 {
    match Opt.wrap(3) { .Some(v) => if v != 3 { return 1 }, .None => return 2 }
    match Opt[Int64].empty() { .Some(_) => return 3, .None => {} }
    0
}
