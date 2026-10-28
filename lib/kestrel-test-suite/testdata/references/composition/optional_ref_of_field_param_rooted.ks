// test: execution
// backends: cranelift,llvm
// stdlib: true

// A ref minted from a FIELD projection of a `mutating` receiver carries
// the receiver's Param root (the field address inherits it), so wrapping
// it in Optional and round-tripping through a var slot (G1 taint) stays
// returnable and aliases the receiver's storage.
module Test

struct Box {
    var v: Int64

    mutating func pick() -> Optional[&Int64] {
        let r = &self.v;
        var o: Optional[&Int64] = .Some(r);
        o
    }
}

@main
func main() {
    var b = Box(v: 5);
    let o = b.pick();
    b.v = 8;
    if let .Some(rv) = o {
        if rv != 8 {
            fatalError("field-rooted optional ref lost aliasing: \(rv)");
        }
    } else {
        fatalError("Some matched as None");
    }
    print("ok");
}

// CHECK: ok
