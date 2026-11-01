// test: execution
// stdlib: true
// expect-exit: 0
//
// An array pattern over a LOCAL array literal. Matching a parameter works
// (`head(xs:)` in the other array_matchable tests); a local `let xs = [...]`
// does not:
// - as a function's tail `match`: OSSA verify ICE ("block arg 0 ... type
//   mismatch"), found 2026-11-01;
// - as `main`'s tail `match`: compiles but exits 15 instead of 0 (Cranelift
//   and LLVM).

module Test

func headLocal() -> Int64 {
    let xs: [Int64] = [5, 6, 7];
    match xs { [a, ..] => a, _ => 9 }
}

@main
func main() -> Int64 {
    if headLocal() != 5 { return 1 }
    let xs: [Int64] = [1, 2, 3];
    match xs { [a, ..] => a - 1, _ => 9 }
}
