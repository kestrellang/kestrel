// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md Open Question 1, decided: Kestrel has no uniqueness
// rule for &mutating and this design adds none — two `mutating` closures over
// the SAME place are permitted. Interleaved calls are deterministic and both
// sets of writes land on `total`.
module Test

@main
func main() -> lang.i64 {
    var total = 0;
    var addOne: mutating () -> () = { total = lang.i64_add(total, 1); };
    var addTen: mutating () -> () = { total = lang.i64_add(total, 10); };
    addOne();
    addTen();
    addOne();
    if lang.i64_eq(total, 12) { } else { return 1; }
    0
}
