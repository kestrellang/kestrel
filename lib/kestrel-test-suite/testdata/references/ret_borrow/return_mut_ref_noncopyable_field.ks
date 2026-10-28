// test: execution
// stdlib: true
// backends: cranelift,llvm

// Non-Copyable fields take the in-place borrow route (no snapshot); the
// returned ref still roots at Param(self) and writes through a member of
// the projected payload are visible in the original.
module Test

struct Payload: not Copyable {
    var n: Int64
}

struct Box: not Copyable {
    var p: Payload

    mutating func view() -> &mutating Payload {
        self.p
    }
}

@main
func main() -> lang.i64 {
    var b = Box(p: Payload(n: 1));
    b.view().n = 5;
    if b.p.n != 5 { return 1; }
    0
}
