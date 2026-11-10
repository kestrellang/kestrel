// Rosetta Code: Stern-Brocot sequence
// https://rosettacode.org/wiki/Stern-Brocot_sequence
module Main

@main
func main() {
    var seq: Array[Int64] = [1, 1];
    var i = 1;
    while seq.count < 1200 {
        seq.append(seq(i) + seq(i - 1));
        seq.append(seq(i));
        i = i + 1;
    }
    println("First 15: \(seq.prefix(15).toOwned())");
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100] {
        println("first \(n:>3) at index \((seq.firstIndex(of: n) ?? -2) + 1)");
    }
    let allCoprime = (1..<1000).iter().all { seq(it - 1).gcd(seq(it)) == 1 };
    println("gcd of consecutive members is 1 for the first 1000: \(allCoprime)");
}
