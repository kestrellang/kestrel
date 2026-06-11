// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: a generic function may DECLARE `-> &H.Item` and forward a
// witness call's ref — ret_borrow threads through both layers (the
// witness call inside the body, and the generic fn's own borrow return
// derived from its declared Ref{AssocProjection} signature). The
// returned ref stays live into the caller: reads see it, and the
// `&mutating` variant writes through.
module Test

protocol Holder {
    type Item
    func fetch() -> &Item
}

protocol MutHolder {
    type Slot
    mutating func access() -> &mutating Slot
}

struct Box: Holder, MutHolder {
    type Item = Int64
    type Slot = Int64
    var v: Int64

    func fetch() -> &Int64 {
        self.v
    }

    mutating func access() -> &mutating Int64 {
        self.v
    }
}

func pass[H](h: H) -> &H.Item where H: Holder {
    h.fetch()
}

func passMut[H](mutating h: H) -> &mutating H.Slot where H: MutHolder {
    h.access()
}

@main
func main() -> lang.i64 {
    var b = Box(v: 5);
    let x = pass(b);
    if x != 5 { return 1; }
    passMut(b) = 9;
    if b.v != 9 { return 2; }
    let y = pass(b);
    if y != 9 { return 3; }
    0
}
