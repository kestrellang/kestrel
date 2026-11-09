// Rosetta Code: Least common multiple
// https://rosettacode.org/wiki/Least_common_multiple
module Main

func lcm(a: Int64, b: Int64) -> Int64 {
    if a == 0 or b == 0 { return 0 }
    (a / a.gcd(b) * b).abs()
}

@main
func main() {
    println("lcm(12, 18) = \(lcm(12, 18))");
    println("lcm(-6, 14) = \(lcm(-6, 14))");
    println("lcm(1..20) = \((1..=20).iter().fold(from: 1, by: { (acc, n) in lcm(acc, n) }))");
    println("std: 12.lcm(18) = \(12.lcm(18))");
}
