// Rosetta Code: Hofstadter Q sequence
// https://rosettacode.org/wiki/Hofstadter_Q_sequence
module Main

@main
func main() {
    let n = 100000;
    var q = Array[Int64](repeating: 1, count: n + 1);
    for i in 3..=n {
        q(i) = q(i - q(i - 1)) + q(i - q(i - 2));
    }
    println("First ten: \(q(1..<11).toOwned())");
    println("1000th: \(q(1000))");
    let drops = (2..=n).iter().filter { q(it) < q(it - 1) }.count();
    println("Terms less than their predecessor up to 100000: \(drops)");
}
