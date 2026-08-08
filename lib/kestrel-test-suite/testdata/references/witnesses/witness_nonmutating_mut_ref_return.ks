// test: execution
// backends: cranelift,llvm
// stdlib: true

// Phase-0 validation for closure-kinds (docs/design/shared-box.md §The
// Protocol): a NON-mutating protocol requirement returning `&mutating Slot`,
// witness-dispatched through a generic body. The conformer derives the
// reference from a Pointer (`valuePtr().mutatingValue`) — the exact shape
// `RcBox.sharedMutRef()` will use. E495 keys on pointer provenance, not on
// the receiver convention, so this must compile and write through.
module Test

protocol SharedMut {
    type Slot
    func sharedAccess() -> &mutating Slot
}

struct Cell: SharedMut {
    type Slot = Int64
    fileprivate var storage: RcBox[Int64]

    public init(v: Int64) {
        self.storage = RcBox(v);
    }

    public func sharedAccess() -> &mutating Int64 {
        self.storage.valuePtr().mutatingValue
    }
}

func writeThrough[H](h: H) where H: SharedMut, H.Slot = Int64 {
    h.sharedAccess() = 42;
}

@main
func main() -> lang.i64 {
    let c = Cell(5);
    writeThrough(c);
    if c.sharedAccess() != 42 { return 1; }
    // copies of the handle observe the same storage
    let d = c;
    d.sharedAccess() = 7;
    if c.sharedAccess() != 7 { return 2; }
    0
}
