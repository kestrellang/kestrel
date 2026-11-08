// Rosetta Code: Currying
// https://rosettacode.org/wiki/Currying
module Main

func add(a: Int64) -> escaping (Int64) -> Int64 {
    { (b: Int64) in a + b }
}

func addThree(a: Int64, b: Int64, c: Int64) -> Int64 { a + b + c }

@main
func main() {
    let add2 = add(2);
    println(add2(3));
    println(add(10)(5));
    // Placeholders give partial application without writing the closure.
    let addTo100 = addThree(100, _, _);
    println(addTo100(20, 3));
}
