// test: diagnostics
// stdlib: true

// A subscript through a shared `&` field is a `&T` place: readable, never
// writable (E208), the same as writing through a bare `&T`.
module Test

struct View { let base: &Array[Int64] }

func poke(v: View) {
    v.base(at: 0) = 5; // ERROR(E208)
}
