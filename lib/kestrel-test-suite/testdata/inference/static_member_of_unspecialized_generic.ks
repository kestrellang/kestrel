// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Calling a static member through an unspecialized generic type
// (`Box.make(3)`) infers the type's parameters from the call, like
// `Box[Int64].make(3)` spells them (the rigid `T` used to leak out).

module Test

struct Box[T] {
    var v: T;
    static func make(v: T) -> Box[T] { Box(v: v) }
}

@main
func main() -> lang.i64 {
    let a = Box.make(3);
    if a.v != 3 { return 1 }
    let b = Box.make("s");
    if b.v.isEqual(to: "s") == false { return 2 }
    let c = Box[Int64].make(4);
    if c.v != 4 { return 3 }
    0
}
