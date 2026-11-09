// Rosetta Code: Self-describing numbers
// https://rosettacode.org/wiki/Self-describing_numbers
module Main

func isSelfDescribing(n: Int64) -> Bool {
    let digits = "\(n)".chars.iter().map { Int64(from: it.digitValue() ?? 0) }.collect();
    if digits.count > 10 { return false }
    (0..<digits.count).iter().all { (i) in digits.iter().filter { it == i }.count() == digits(i) }
}

@main
func main() {
    for n in [1210, 2020, 21200, 3211000, 42101000, 1211, 12345] {
        println("\(n): \(isSelfDescribing(n))");
    }
}
