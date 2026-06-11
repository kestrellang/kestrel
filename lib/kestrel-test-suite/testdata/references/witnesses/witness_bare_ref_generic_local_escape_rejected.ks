// test: diagnostics
// stdlib: true

// Stage 2d: ret_borrow escape checking applies to generic `-> &H.Item`
// bodies pre-mono — a witness-returned ref rooted at a LOCAL copy of
// the holder cannot escape the frame.
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

func bad[H](h: H) -> &H.Item where H: Holder {
    var mine = h;
    mine.fetch() // ERROR(E494)
}

@main
func main() {
    let b = Box(v: 1);
    let x = bad(b);
}
