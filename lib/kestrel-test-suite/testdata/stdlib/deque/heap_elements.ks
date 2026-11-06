// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test

import std.collections.(Deque)

// Deque over a heap-owning element type: push/pop at both ends, iteration,
// and the caller's values surviving. Every other deque test uses Int64, so a
// push that aliased the caller's String (crash on pop/iterate) went unseen.

@main
func main() -> Int64 {
    var d = Deque[String]();
    let x = "x-\(1)";
    d.pushBack(x);
    d.pushFront("y-\(2)");
    d.pushBack("z-\(3)");
    if x != "x-1" { return 1 }

    var joined = "";
    for s in d { joined = joined + s + ","; }
    if joined != "y-2,x-1,z-3," { return 2 }

    if (d.popFront() ?? "") != "y-2" { return 3 }
    if (d.popBack() ?? "") != "z-3" { return 4 }
    if (d.popFront() ?? "") != "x-1" { return 5 }
    if d.count != 0 { return 6 }
    if let .Some(_) = d.popBack() { return 7 }

    // Many pushes through growth, then drain.
    for i in 0..<100 { d.pushBack("n-\(i)"); }
    var total = 0;
    while let .Some(s) = d.popFront() { total = total + s.bytes.count; }
    if total != 390 { return 8 }  // 10 × "n-D" (3 bytes) + 90 × "n-DD" (4 bytes)
    0
}
