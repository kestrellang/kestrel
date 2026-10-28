// test: diagnostics
// stdlib: true

// Stage 2d: ref-Item `next()` bodies stay escape-checked (pre-mono
// carrier mode): wrapping a ref to a LOCAL into the returned
// Optional[&T] is E494 exactly like any owned carrier return.
module Test

struct BadIter {
    var idx: Int64
}

extend BadIter: Iterator {
    type Item = &Int64

    mutating func next() -> Optional[&Int64] {
        var local = 7;
        let r = &local;
        let o: Optional[&Int64] = .Some(r);
        o // ERROR(E494)
    }
}

@main
func main() {
    var it = BadIter(idx: 0);
    let x = it.next();
}
