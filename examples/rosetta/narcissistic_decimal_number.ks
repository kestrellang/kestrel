// Rosetta Code: Narcissistic decimal number
// https://rosettacode.org/wiki/Narcissistic_decimal_number
module Main

func isNarcissistic(n: Int64) -> Bool {
    var digits = Array[Int64]();
    var rest = n;
    loop {
        digits.append(rest % 10);
        rest = rest / 10;
        if rest == 0 { break }
    }
    let power = digits.count;
    digits.iter().map { it.pow(power) }.sum() == n
}

@main
func main() {
    let found = (0..).iter().filter { isNarcissistic(it) }.take(25).collect();
    println(found);
}
