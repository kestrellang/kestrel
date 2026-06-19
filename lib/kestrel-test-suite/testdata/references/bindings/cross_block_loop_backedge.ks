// test: execution
// backends: cranelift,llvm
// stdlib: true

// A binding threads through loop headers and back-edges (and `break` from
// a nested `if`), observing writes to the borrowed var on every iteration.

module Test

@main
func main() {
    var x = 3;
    let r = &x;
    var total = 0;
    var i = 0;
    while i < 4 {
        total = total + r;  // 3 + 4 + 5 + 6
        x = x + 1;
        i = i + 1;
    }
    if total != 18 {
        fatalError("loop back-edge threading broke: \(total)");
    }

    var acc = 0;
    var j = 0;
    while true {
        if j >= 3 { break; }
        acc = acc + r;      // x is 7 now: 7*3
        j = j + 1;
    }
    if acc != 21 {
        fatalError("break threading broke: \(acc)");
    }
    print("ok");
}

// CHECK: ok
