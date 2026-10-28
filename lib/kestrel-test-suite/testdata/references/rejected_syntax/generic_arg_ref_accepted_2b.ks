// test: diagnostics
// stdlib: false

// Stage 2b: a ref TYPE ARGUMENT is a legal formation. (With the stdlib
// present, an UNRELAXED owner's implicit `T: Static` bound rejects the
// instantiation — see references/composition/heap_of_ref_rejected; this
// no-stdlib fixture has no Static builtin, so formation alone is pinned.)
module Test

struct Box[T] {
    var v: T
}

func f(b: Box[&lang.i64]) { }
