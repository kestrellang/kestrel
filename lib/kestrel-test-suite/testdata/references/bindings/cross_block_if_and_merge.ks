// test: execution
// backends: cranelift,llvm
// stdlib: true

// References "1.75": a named ref binding stays live across `if` arms and
// merges — threaded as a @guaranteed block arg through every boundary.
// Reads inside arms, after the merge, and after a may-alias write through
// the original var all see the place.

module Test

@main
func main() {
    var x = 10;
    let r = &x;
    var total = 0;
    if x > 5 {
        total = total + r;      // in-arm read: 10
    } else {
        total = 100;
    }
    total = total + r;          // post-merge read: 20
    x = 20;                     // may-alias write, visible through r
    total = total + r;          // 40
    if total != 40 {
        fatalError("cross-block binding reads broke: \(total)");
    }
    print("ok");
}

// CHECK: ok
