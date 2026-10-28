// test: diagnostics

// A normal closure that captures a local is a view into this frame and
// cannot be returned, whatever path it takes to the return. Each of these
// paths used to lose track of the capture, so the closure escaped and was
// called after its frame was gone.

module Test

// Inside an enum payload.
func inOptional(n: Int64) -> Optional[() -> Int64] {
    let k: Int64 = n * 3;
    .Some({ k }) // ERROR(E494)
}

// As the value of an if/else.
func ifJoin(n: Int64) -> () -> Int64 {
    let k: Int64 = n * 3;
    let f = { k };
    if n > 0 { f } else { { 0 } } // ERROR(E494)
}

// Reassigned in a branch, then returned.
func branchReassign(n: Int64) -> () -> Int64 {
    let k: Int64 = n * 3;
    var f: () -> Int64 = { 1 };
    if n > 0 { f = { k }; }
    f // ERROR(E494)
}

// Reassigned in a loop, then returned.
func loopReassign(n: Int64) -> () -> Int64 {
    let k: Int64 = n * 3;
    var f: () -> Int64 = { 1 };
    for i in 0..<2 { f = { k }; }
    f // ERROR(E494)
}

// Through a call that may return its argument.
func pick(a: () -> Int64, b: () -> Int64) -> () -> Int64 { a }
func throughCall(n: Int64) -> () -> Int64 {
    let k: Int64 = n * 3;
    pick({ k }, { 0 }) // ERROR(E494)
}
