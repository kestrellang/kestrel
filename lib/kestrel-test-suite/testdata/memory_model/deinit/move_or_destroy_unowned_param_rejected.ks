// test: diagnostics

// A borrowed (default) parameter still belongs to the caller: moving it out
// or destroying it would release the caller's value a second time. A
// `mutating` one may be moved out only if a value is put back before every
// return. A `consuming` parameter is the body's to move or `deinit`.

module Test

struct R: not Copyable {
    var id: Int64;
    deinit { }
}

func take(consuming r: R) { }

func moveBorrowed(r: R) {
    take(r); // ERROR: cannot move parameter 'r'
}

func moveMutating(mutating r: R) { // ERROR: mutating parameter 'r' is moved out and not replaced
    take(r);
}

func swapMutating(mutating r: R) {
    take(r);
    r = R(id: 2);
}

func destroyBorrowed(r: R) {
    deinit r; // ERROR: cannot destroy parameter 'r'
}

func fine(consuming r: R) {
    take(r);
}
