// test: diagnostics
// stdlib: true

// A method's ref-carrying result may borrow a non-receiver argument whose
// type can hold what it points at, so returning it is rejected when that
// argument is a local.
module Test

struct View { let base: &Array[Int64] }

extend Array[Int64] {
    func other(o: [Int64]) -> View { let b = &o; View(base: b) }
}

func leak(xs: [Int64]) -> View {
    let local: [Int64] = [4];
    xs.other(local) // ERROR(E494)
}
