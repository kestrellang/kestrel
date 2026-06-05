// test: execution
// stdlib: false
// expect-exit: 2

// A direct call on a concrete `Box[lang.i64]` must dispatch to the most
// specific *inherent* extension (`Box[lang.i64]`, marker 2), not the generic
// `Box[T]` (marker 1). This exercises the analyzer's member-resolution
// selector (resolve.rs / TypeMembers) for a non-polymorphic call.
module Main

struct Box[T] { var value: T }

extend Box[T]        { func mark() -> lang.i64 { 1 } }
extend Box[lang.i64] { func mark() -> lang.i64 { 2 } }

@main
func main() -> lang.i64 {
    let b = Box[lang.i64](value: 0);
    b.mark() // most specific applicable extension wins -> 2
}
