// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Opt.Some(x)`: bare `Opt` is `Opt[_]`, inferred from the payload.

enum Opt[T] { case Some(T) case None }

@main
func main() -> Int64 {
    let o = Opt.Some("hi");
    match o { .Some(v) => if v != "hi" { return 1 }, .None => return 2 }
    0
}
