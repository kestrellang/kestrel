// Rosetta Code: Mutual recursion (Hofstadter Female and Male sequences)
// https://rosettacode.org/wiki/Mutual_recursion
module Main

func f(n: Int64) -> Int64 { if n == 0 { 1 } else { n - m(f(n - 1)) } }
func m(n: Int64) -> Int64 { if n == 0 { 0 } else { n - f(m(n - 1)) } }

@main
func main() {
    println("F: \((0..<20).iter().map { f(it) }.collect())");
    println("M: \((0..<20).iter().map { m(it) }.collect())");
}
