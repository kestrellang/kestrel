// test: diagnostics
// stdlib: true

// Storing a method result that may borrow an inner local ARGUMENT (not just
// the receiver) into an outer binding is E509.
module Test

struct View { let base: &Array[Int64] }

extend Array[Int64] {
    func other(o: [Int64]) -> View { let b = &o; View(base: b) }
}

@main
func main() {
    let xs: [Int64] = [1];
    var o = xs.other(xs);
    if true {
        let inner: [Int64] = [5];
        o = xs.other(inner); // ERROR(E509)
    }
    println("\(o.base(0))");
}
