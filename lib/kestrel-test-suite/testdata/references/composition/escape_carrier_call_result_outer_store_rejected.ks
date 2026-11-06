// test: diagnostics
// stdlib: true

// A carrier returned by a function borrows its arguments: storing it into an
// outer binding while the argument is an inner local is E509 (it used to
// compile and read freed memory).
module Test

struct View { let base: &Array[Int64] }

func mk(xs: [Int64]) -> View {
    let b = &xs;
    View(base: b)
}

@main
func main() {
    let zs: [Int64] = [0];
    var o = mk(zs);
    if true {
        let xs: [Int64] = [1, 2, 3];
        o = mk(xs); // ERROR(E509)
    }
    println("\(o.base(0))");
}
