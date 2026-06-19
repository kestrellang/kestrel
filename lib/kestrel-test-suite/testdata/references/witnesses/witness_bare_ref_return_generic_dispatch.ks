// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: a bare-ref protocol requirement (`func fetch() -> &Item`)
// dispatches through a witness in a GENERIC body. The witness call is a
// ret_borrow call — the caller derives the convention from the protocol
// method's declared return (the E458 exact-shape rule guarantees every
// impl agrees). Before the Callee::Witness arm existed, the returned
// raw pointer registered as an owned value and the pointer BITS read
// back as the pointee.
module Test

protocol Holder {
    type Item
    func fetch() -> &Item
}

struct Box: Holder {
    type Item = Int64
    var v: Int64

    func fetch() -> &Int64 {
        self.v
    }
}

func readThrough[H](h: H) -> Int64 where H: Holder, H.Item = Int64 {
    let x = h.fetch();
    x
}

@main
func main() -> lang.i64 {
    let b = Box(v: 5);
    if readThrough(b) != 5 { return 1; }
    let big = Box(v: 123456789);
    if readThrough(big) != 123456789 { return 2; }
    0
}
