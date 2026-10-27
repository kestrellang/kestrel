// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A long chain of dependent operators (300 nested negations, past the old 256 cap) needs one solver
// round per link. The round cap now scales with the body; a fixed 256 rounds
// rejected this valid code with a bogus type mismatch.

module Test

@main
func main() -> lang.i64 {
    let x: Int64 = 3;
    let y = ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------x;
    if y != 3 { return 1 }
    0
}
