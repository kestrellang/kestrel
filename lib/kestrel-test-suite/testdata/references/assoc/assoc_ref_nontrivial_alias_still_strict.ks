// test: diagnostics
// stdlib: true

// Stage 2d: only TRIVIAL member aliases get the ref carve. A
// parameterized alias flows as AliasUse through solver Reduce with no
// use-site position re-check, so its RHS stays Strict.
module Test

struct S {
    type Pair[U] = Optional[&U] // ERROR(E485)
    var v: Int64
}
