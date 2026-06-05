// test: execution
// stdlib: false
// expect-exit: 7

// The unit type `()` is extendable and can conform to a protocol. Dispatch
// works both directly (`u.tag()`) and through a generic function (witness
// dispatch). `()` has no nominal entity of its own, so this exercises the
// synthetic `lang.()` entity wired through name-res, inference conformance, and
// witness lowering (implementing type = `MirTy::Tuple([])`).
module Main

protocol Tag { func tag() -> lang.i64 }

extend (): Tag { func tag() -> lang.i64 { 7 } }

func tagOf[T](x: T) -> lang.i64 where T: Tag { x.tag() }

@main
func main() -> lang.i64 {
    let u = ();
    if lang.i64_ne(u.tag(), 7) { return 1; } // direct call
    tagOf(u)                                  // witness dispatch
}
