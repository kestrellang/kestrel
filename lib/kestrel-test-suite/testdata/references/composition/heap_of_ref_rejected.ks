// test: diagnostics
// stdlib: true

// References 2b containment: heap containers keep their implicit Static
// bound — a ref (or ref-bearing aggregate) type argument fails it.
module Test

@main
func main() {
    let xs: Array[&Int64] = Array(); // ERROR: !: Static
    let ys: Array[Optional[&Int64]] = Array(); // ERROR: !: Static
}
