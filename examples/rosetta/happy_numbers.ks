// Rosetta Code: Happy numbers
// https://rosettacode.org/wiki/Happy_numbers
module Main

func digitSquareSum(n: Int64) -> Int64 {
    var rest = n;
    var sum = 0;
    while rest > 0 {
        let d = rest % 10;
        sum = sum + d * d;
        rest = rest / 10;
    }
    sum
}

func isHappy(n: Int64) -> Bool {
    var seen = Set[Int64]();
    var x = n;
    while x != 1 and seen.insert(x) {
        x = digitSquareSum(x);
    }
    x == 1
}

@main
func main() {
    println((1..).iter().filter { isHappy(it) }.take(8).collect());
}
