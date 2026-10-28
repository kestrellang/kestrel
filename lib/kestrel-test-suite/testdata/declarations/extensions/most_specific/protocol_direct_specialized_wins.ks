// test: execution
// stdlib: false
// expect-exit: 2

// Like the inherent case, but the overloaded method comes from a *protocol
// conformance*. A direct call on a concrete `Box[lang.i64]` must select the
// specialized conformance `extend Box[lang.i64]: Marked` (2) over the generic
// `extend Box[T]: Marked` (1). Still the analyzer's selector (the call is
// monomorphic, so no witness dispatch yet).
module Main

protocol Marked { func mark() -> lang.i64 }

struct Box[T] { var value: T }

extend Box[T]:        Marked { func mark() -> lang.i64 { 1 } }
extend Box[lang.i64]: Marked { func mark() -> lang.i64 { 2 } }

@main
func main() -> lang.i64 {
    let b = Box[lang.i64](value: 0);
    b.mark() // specialized conformance wins -> 2
}
