// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `t.1.0` indexes twice. The lexer reads `1.0` as one float literal, so it is
// split back into `1 . 0` when it directly follows a member `.`.

module Test

@main
func main() -> lang.i64 {
    let t = (1, (2, (3, 4)));
    if t.1.0 != 2 { return 1 }
    if t.1.1.1 != 4 { return 2 }
    let s = "\(t.1.0)";
    if s.isEqual(to: "2") == false { return 3 }
    let f = 1.5;
    if f != 1.5 { return 4 }
    0
}
