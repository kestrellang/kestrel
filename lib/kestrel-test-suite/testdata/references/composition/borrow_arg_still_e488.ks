// test: diagnostics
// stdlib: true

// References 2b: a syntactic `&x` ARGUMENT is still illegal (E488) — ref
// payloads are built from named bindings or ref-returning calls, so the
// borrow's lifetime has a name.
module Test

func f() {
    var x = 1;
    let o: Optional[&Int64] = .Some(&x); // ERROR(E488)
}
