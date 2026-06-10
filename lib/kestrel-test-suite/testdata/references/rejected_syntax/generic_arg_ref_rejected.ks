// test: diagnostics
// stdlib: false

// Stage 0.5: a ref as a generic type argument is storage-by-the-back-door
// (E-REF-19 in stage 1 guards the inferred path; this is the annotated one).
module Test

struct Box[T] {
    var v: T
}

func f(b: Box[&lang.i64]) { } // ERROR(E485)
