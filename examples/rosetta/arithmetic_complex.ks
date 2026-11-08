// Rosetta Code: Arithmetic/Complex
// https://rosettacode.org/wiki/Arithmetic/Complex
module Main

struct Complex {
    let re: Float64;
    let im: Float64;

    func conjugate() -> Complex { Complex(re: self.re, im: -self.im) }

    func inverse() -> Complex {
        let d = self.re * self.re + self.im * self.im;
        Complex(re: self.re / d, im: -self.im / d)
    }
}

// Conforming to the operator protocols gives `+`, `*` and unary `-`.
extend Complex: Addable {
    type Output = Complex;
    static var zero: Complex { get { Complex(re: 0.0, im: 0.0) } }
    consuming func add(consuming other: Complex) -> Complex {
        Complex(re: self.re + other.re, im: self.im + other.im)
    }
}

extend Complex: Multipliable {
    type Output = Complex;
    static var one: Complex { get { Complex(re: 1.0, im: 0.0) } }
    consuming func multiply(consuming other: Complex) -> Complex {
        Complex(re: self.re * other.re - self.im * other.im,
                im: self.re * other.im + self.im * other.re)
    }
}

extend Complex: Negatable {
    type Output = Complex;
    consuming func negate() -> Complex { Complex(re: -self.re, im: -self.im) }
}

extend Complex: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        if self.im < 0.0 {
            writer.append("\(self.re) - \(-self.im)i");
        } else {
            writer.append("\(self.re) + \(self.im)i");
        }
    }
}

@main
func main() {
    let a = Complex(re: 1.0, im: 1.0);
    let b = Complex(re: 3.14159, im: 1.25);
    println("a       = \(a)");
    println("b       = \(b)");
    println("a + b   = \(a + b)");
    println("a * b   = \(a * b)");
    println("-a      = \(-a)");
    println("1 / a   = \(a.inverse())");
    println("conj(a) = \(a.conjugate())");
}
