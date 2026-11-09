// Rosetta Code: Kaprekar numbers
// https://rosettacode.org/wiki/Kaprekar_numbers
module Main

func isKaprekar(n: Int64) -> Bool {
    if n == 1 { return true }
    let square = n * n;
    var split: Int64 = 10;
    while split <= square {
        let (left, right) = (square / split, square % split);
        if right > 0 and left + right == n { return true }
        split = split * 10;
    }
    false
}

@main
func main() {
    let below10k = (1..<10000).iter().filter { isKaprekar(it) }.collect();
    println(below10k);
    println("Kaprekar numbers below one million: \((1..<1000000).iter().filter { isKaprekar(it) }.count())");
}
