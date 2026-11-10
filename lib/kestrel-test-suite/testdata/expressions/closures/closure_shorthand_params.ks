// test: execution
// stdlib: true
// expect-stdout: 10 30\n7\n12\n4\n6\n7\n6\n

// Untyped closure parameters can drop the parentheses: `{ c in … }`,
// `{ a, b in … }`, and `_` for an unused one (`{ _ in 7 }`).
module Test

func apply(f: (Int64) -> Int64) -> Int64 { f(5) }
func twice(f: (Int64, Int64) -> Int64) -> Int64 { f(3, 4) }

@main
func main() -> Int64 {
    let xs = [1, 2, 3];
    let ys = xs.map { c in c * 10 };
    println("\(ys(0)) \(ys(2))");
    println("\(apply { _ in 7 })");
    println("\(twice { a, b in a * b })");
    println("\(twice { _, b in b })");
    println("\(apply({ x in x + 1 }))");
    let f = { n in n + 2 };
    println("\(apply(f))");
    let total = xs.iter().fold(from: 0) { acc, x in acc + x };
    println("\(total)");
    0
}
