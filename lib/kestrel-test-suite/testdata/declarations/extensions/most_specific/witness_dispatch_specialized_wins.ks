// test: execution
// stdlib: false
// expect-exit: 2

// THE CORE CASE. Calling through a generic function makes `x.mark()` a witness
// dispatch resolved at monomorphization (Callee::Witness ->
// kestrel-mir/src/mono/witness.rs find_witness_with_method). For
// `Box[lang.i64]` BOTH `extend Box[T]: Marked` (1) and
// `extend Box[lang.i64]: Marked` (2) pattern-match the self type, because
// match_pattern treats the generic's `T` as a wildcard. The selector currently
// returns the FIRST match (no specificity ordering), so it picks the generic.
// Most-specific-wins requires choosing the specialized conformance -> 2.
module Main

protocol Marked { func mark() -> lang.i64 }

struct Box[T] { var value: T }

extend Box[T]:        Marked { func mark() -> lang.i64 { 1 } }
extend Box[lang.i64]: Marked { func mark() -> lang.i64 { 2 } }

func markOf[T](x: T) -> lang.i64 where T: Marked { x.mark() }

@main
func main() -> lang.i64 {
    let b = Box[lang.i64](value: 0);
    markOf(b) // witness dispatch must pick the specialized conformance -> 2
}
