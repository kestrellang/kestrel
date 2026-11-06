// test: execution
// stdlib: true
// backends: cranelift,llvm

// A subscript lends its element, so the call keeps its receiver and index
// borrows open for the returned ref. Those borrows must end with the ref,
// not at scope exit: moving the array out afterwards (returning it) is
// legal once the element read or write is done. Regression for the OSSA
// verifier ICE "cannot take from ...: active borrow(s) ... depend on it"
// in BLAKE2b.initialState.
module Test

// Read through the subscript, then move the base out.
func readThenReturn() -> Array[Int64] {
    var h: Array[Int64] = [1, 2];
    let x = h(0);
    h
}

// Write through the subscript, then move the base out.
func writeThenReturn() -> Array[Int64] {
    var h: Array[Int64] = [1, 2];
    let v: Int64 = 5;
    h(1) = v;
    h
}

// Read and write in one statement, then move the base out (the
// BLAKE2b.initialState shape).
func readWriteThenReturn(n: Int64) -> Array[Int64] {
    var h: Array[Int64] = [16, 2];
    h(0) = h(0) ^ 3 ^ n;
    h
}

// A `let` base: no write, only the lent read.
func letReadThenReturn() -> Array[Int64] {
    let h: Array[Int64] = [7, 8];
    let x = h(1);
    h
}

@main
func main() -> lang.i64 {
    let a = readThenReturn();
    if a.count != 2 or a(0) != 1 { return 1; }
    let b = writeThenReturn();
    if b(1) != 5 { return 2; }
    let c = readWriteThenReturn(32);
    if c(0) != 51 { return 3; }
    let d = letReadThenReturn();
    if d(0) != 7 { return 4; }
    0
}
