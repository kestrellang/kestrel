// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Opt[Int64].None` as a value: no payload, so the written owner args are the
// only source of `T`.

enum Opt[T] { case Some(T) case None }

func isNone(o: Opt[Int64]) -> Bool {
    match o { .Some(_) => false, .None => true }
}

@main
func main() -> Int64 {
    let o = Opt[Int64].None;
    if not isNone(o) { return 1 }
    0
}
