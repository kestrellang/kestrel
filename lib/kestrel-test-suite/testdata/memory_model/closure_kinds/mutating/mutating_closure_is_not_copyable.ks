// test: diagnostics
// stdlib: false

// docs/design/closures.md: a `mutating`-kind closure value is `not Copyable` —
// that is what keeps write-back sound with no aliasing analysis. Binding it to
// a second name is therefore a MOVE, and the original is dead afterwards.
module Test

func test() {
    var total = 0;
    var first: mutating () -> () = { total = lang.i64_add(total, 1); };
    var second = first;
    second();
    first(); // ERROR(E500)
}
