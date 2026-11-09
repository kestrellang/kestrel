// Rosetta Code: Arithmetic/Rational
// https://rosettacode.org/wiki/Arithmetic/Rational
module Main

// A fraction kept in lowest terms with a positive denominator.
struct Rational {
    let num: Int64;
    let den: Int64;

    init(n: Int64, d: Int64) {
        let g = n.gcd(d).abs();
        let sign: Int64 = if d < 0 { -1 } else { 1 };
        self.num = sign * n / g;
        self.den = sign * d / g;
    }
}

extend Rational: Addable {
    type Output = Rational;
    static var zero: Rational { get { Rational(0, 1) } }
    consuming func add(consuming o: Rational) -> Rational {
        Rational(self.num * o.den + o.num * self.den, self.den * o.den)
    }
}

extend Rational: Subtractable {
    type Output = Rational;
    consuming func subtract(consuming o: Rational) -> Rational {
        Rational(self.num * o.den - o.num * self.den, self.den * o.den)
    }
}

extend Rational: Multipliable {
    type Output = Rational;
    static var one: Rational { get { Rational(1, 1) } }
    consuming func multiply(consuming o: Rational) -> Rational {
        Rational(self.num * o.num, self.den * o.den)
    }
}

extend Rational: Divisible {
    type Output = Rational;
    consuming func divide(consuming o: Rational) -> Rational {
        Rational(self.num * o.den, self.den * o.num)
    }
}

extend Rational: Equatable {
    func isEqual(to o: Rational) -> Bool { self.num == o.num and self.den == o.den }
}

extend Rational: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        writer.append("\(self.num)/\(self.den)");
    }
}

// The task: find perfect numbers by checking that the sum of 1/d over all
// divisors d (including n itself) is exactly 2.
@main
func main() {
    let a = Rational(1, 2);
    let b = Rational(-3, 4);
    println("\(a) + \(b) = \(a + b)");
    println("\(a) - \(b) = \(a - b)");
    println("\(a) * \(b) = \(a * b)");
    println("\(a) / \(b) = \(a / b)");

    for candidate in 2..<(1 << 19) {
        var sum = Rational(1, candidate);
        var f: Int64 = 2;
        while f * f <= candidate {
            if candidate % f == 0 {
                sum = sum + Rational(1, f) + Rational(1, candidate / f);
            }
            f = f + 1;
        }
        if sum == Rational(1, 1) {
            println("Sum of reciprocal factors of \(candidate) = 1 exactly: perfect");
        }
    }
}
