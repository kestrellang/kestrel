// Rosetta Code: Long multiplication
// https://rosettacode.org/wiki/Long_multiplication
module Main

// Schoolbook multiplication on decimal digit strings.
func multiply(a: String, b: String) -> String {
    let x = a.chars.reversed.iter().map { Int64(from: it.digitValue() ?? 0) }.collect();
    let y = b.chars.reversed.iter().map { Int64(from: it.digitValue() ?? 0) }.collect();
    var digits = Array[Int64](repeating: 0, count: x.count + y.count);
    for i in 0..<x.count {
        var carry = 0;
        for j in 0..<y.count {
            let t = digits(i + j) + x(i) * y(j) + carry;
            digits(i + j) = t % 10;
            carry = t / 10;
        }
        digits(i + y.count) = digits(i + y.count) + carry;
    }
    while digits.count > 1 and digits.last == .Some(0) { digits.pop(); }
    digits.reversed.iter().map { "\(it)" }.collect().joined(by: "")
}

@main
func main() {
    // 2^64 * 2^64 = 2^128
    println(multiply("18446744073709551616", "18446744073709551616"));
}
