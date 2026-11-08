// Rosetta Code: Factorial (100! with a hand-rolled bignum)
// https://rosettacode.org/wiki/Factorial
module Main

// Little-endian base-10^9 limbs.
struct BigNat {
    var limbs: Array[Int64];

    init(small n: Int64) { self.limbs = [n]; }

    mutating func multiply(by m: Int64) {
        var carry: Int64 = 0;
        for i in 0..<self.limbs.count {
            let t = self.limbs(i) * m + carry;
            self.limbs(i) = t % 1000000000;
            carry = t / 1000000000;
        }
        while carry > 0 {
            self.limbs.append(carry % 1000000000);
            carry = carry / 1000000000;
        }
    }
}

extend BigNat: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        var i = self.limbs.count - 1;
        writer.append("\(self.limbs(i))");
        i = i - 1;
        while i >= 0 {
            writer.append("\(self.limbs(i):09)");
            i = i - 1;
        }
    }
}

@main
func main() {
    var f = BigNat(small: 1);
    for n in 2..=100 { f.multiply(by: n); }
    let digits = "\(f)";
    println("100! = \(digits)");
    let digitSum = digits.chars.iter().map { Int64(from: it.digitValue() ?? 0) }.sum();
    println("\(digits.chars.count) digits, digit sum \(digitSum)");
}
