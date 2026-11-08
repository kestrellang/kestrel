// Rosetta Code: Function composition
// https://rosettacode.org/wiki/Function_composition
module Main

func compose[A, B, C](f: escaping (B) -> C, g: escaping (A) -> B) -> escaping (A) -> C {
    { (x: A) in f(g(x)) }
}

@main
func main() {
    let sinOfAsin = compose({ (x: Float64) in x.sin() }, { (x: Float64) in x.asin() });
    println(sinOfAsin(0.5));
    let addThenDouble = compose({ (x: Int64) in x * 2 }, { (x: Int64) in x + 1 });
    println(addThenDouble(20));
}
