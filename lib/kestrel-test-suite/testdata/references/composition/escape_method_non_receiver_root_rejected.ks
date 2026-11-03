// test: diagnostics
// stdlib: true

// A method's returned references must borrow from its receiver (E512).
// Callers root a method-made carrier at the receiver alone, so a method
// that returns a borrow of another parameter would let it dangle.
module Test

struct View { let base: &Array[Int64] }

extend Array[Int64] {
    func other(o: [Int64]) -> View {
        let b = &o;
        View(base: b) // ERROR(E512)
    }
}
