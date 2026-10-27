// test: diagnostics

// Aliases are transparent, so an alias that names itself — directly, through
// another alias, or inside a type argument — is an infinite type. Using one
// used to overflow the compiler's stack.

module Test

struct S[T] { var t: T; }

type A = B; // ERROR: circular type alias
type B = A;

type C = Array[C]; // ERROR: circular type alias

type D = S[D]; // ERROR: circular type alias

func use(a: A, c: C, d: D) { }
