// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Opt[Int64].Some(1)`: `[Int64]` binds the enum's `T`.

enum Opt[T] { case Some(T) case None }

@main
func main() -> Int64 {
    let o = Opt[Int64].Some(1);
    match o { .Some(v) => if v != 1 { return 1 }, .None => return 2 }
    0
}
