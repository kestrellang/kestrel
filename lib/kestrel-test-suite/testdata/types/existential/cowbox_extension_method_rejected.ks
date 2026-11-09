// test: diagnostics
// stdlib: true
//
// E146: `any[CowBox] P` needs `P: Cloneable`, and `Cloneable.clone()`
// returns `Self`, which can't be called through an existential. So
// `any[CowBox] P: P` doesn't hold yet, and an extension method of `P`
// (which runs with the box as `Self`) is rejected with a message that names
// the gap. Requirements themselves still go through the table.

module Test

protocol HasN: Cloneable { var n: Int64 { get } }

extend HasN {
    func doubled() -> Int64 { self.n * 2 }
}

struct T: HasN, Cloneable {
    var n: Int64
    func clone() -> T { T(n: self.n) }
}

@main
func main() -> Int64 {
    let c: any[CowBox] HasN = T(n: 3);
    let a = c.n;
    let b = c.doubled(); // ERROR: 'any[CowBox] HasN' does not conform to 'HasN': 'HasN' refines 'Cloneable', whose 'clone() -> Self' cannot be called through an existential yet, so extension methods and generic code over 'HasN' are not available on 'any[CowBox] HasN'
    a + b
}
