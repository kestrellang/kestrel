// Rosetta Code: Ackermann function
// https://rosettacode.org/wiki/Ackermann_function
module Main

func ackermann(m: Int64, n: Int64) -> Int64 {
    if m == 0 { return n + 1 }
    if n == 0 { return ackermann(m - 1, 1) }
    ackermann(m - 1, ackermann(m, n - 1))
}

@main
func main() {
    for m in 0..=3 {
        for n in 0..=4 {
            println("A(\(m), \(n)) = \(ackermann(m, n))");
        }
    }
}
