// test: execution
// stdlib: true
// expect-exit: 0

// A `&mutating` payload binder on a `mutating` receiver borrows the
// receiver's storage in place, so the reference can be returned (the
// stdlib's `chainMutate` relies on this). The pinned scrutinee address keeps
// the receiver's provenance through the match's block arguments.

module Test
enum Opt { case Some(Int64)  case None }
extend Opt {
    mutating func m() -> Int64 {
        match self {
            .Some(&mutating v) => { v = 5; 1 },
            .None => 0
        }
    }
    mutating func r() -> Optional[&mutating Int64] {
        match self {
            .Some(&mutating v) => { let o: Optional[&mutating Int64] = .Some(v); o },
            .None => .None
        }
    }
}
@main
func main() -> lang.i64 {
    var x = Opt.Some(1);
    x.m();
    let a = match x { .Some(v) => v, .None => 9 };
    if a != 5 { return 1 }
    if let .Some(w) = x.r() { w = 7; }
    let b = match x { .Some(v) => v, .None => 9 };
    if b != 7 { return 2 }
    0
}
