// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: a `-> &mutating Slot` requirement witness-dispatched in a
// generic body supports store-through — the write lands in the caller's
// storage.
module Test

protocol MutHolder {
    type Slot
    mutating func access() -> &mutating Slot
}

struct Box: MutHolder {
    type Slot = Int64
    var v: Int64

    mutating func access() -> &mutating Int64 {
        self.v
    }
}

func writeThrough[H](mutating h: H) where H: MutHolder, H.Slot = Int64 {
    h.access() = 42;
}

func bumpThrough[H](mutating h: H) where H: MutHolder, H.Slot = Int64 {
    h.access() += 1;
}

@main
func main() -> lang.i64 {
    var b = Box(v: 5);
    writeThrough(b);
    if b.v != 42 { return 1; }
    bumpThrough(b);
    if b.v != 43 { return 2; }
    0
}
