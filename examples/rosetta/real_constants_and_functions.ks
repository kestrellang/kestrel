// Rosetta Code: Real constants and functions
// https://rosettacode.org/wiki/Real_constants_and_functions
module Main

@main
func main() {
    let x = 2.5;
    println("e = \(Float64.e)");
    println("pi = \(Float64.pi)");
    println("sqrt(x) = \(x.sqrt())");
    println("ln(x) = \(x.ln())");
    println("log10(x) = \(x.log10())");
    println("exp(x) = \(x.exp())");
    println("abs(-x) = \((-x).abs())");
    println("floor(x) = \(x.floor()), ceil(x) = \(x.ceil())");
    println("x ^ 3.0 = \(x.pow(3.0))");
}
