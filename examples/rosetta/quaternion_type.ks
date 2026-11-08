// Rosetta Code: Quaternion type
// https://rosettacode.org/wiki/Quaternion_type
module Main

struct Quaternion {
    let a: Float64;
    let b: Float64;
    let c: Float64;
    let d: Float64;

    func norm() -> Float64 { (self.a * self.a + self.b * self.b + self.c * self.c + self.d * self.d).sqrt() }
    func conjugate() -> Quaternion { Quaternion(a: self.a, b: -self.b, c: -self.c, d: -self.d) }
    func plus(r: Float64) -> Quaternion { Quaternion(a: self.a + r, b: self.b, c: self.c, d: self.d) }
    func times(r: Float64) -> Quaternion { Quaternion(a: self.a * r, b: self.b * r, c: self.c * r, d: self.d * r) }
}

extend Quaternion: Addable {
    type Output = Quaternion;
    static var zero: Quaternion { get { Quaternion(a: 0.0, b: 0.0, c: 0.0, d: 0.0) } }
    consuming func add(consuming q: Quaternion) -> Quaternion {
        Quaternion(a: self.a + q.a, b: self.b + q.b, c: self.c + q.c, d: self.d + q.d)
    }
}

extend Quaternion: Multipliable {
    type Output = Quaternion;
    static var one: Quaternion { get { Quaternion(a: 1.0, b: 0.0, c: 0.0, d: 0.0) } }
    // The Hamilton product: not commutative.
    consuming func multiply(consuming q: Quaternion) -> Quaternion {
        Quaternion(
            a: self.a * q.a - self.b * q.b - self.c * q.c - self.d * q.d,
            b: self.a * q.b + self.b * q.a + self.c * q.d - self.d * q.c,
            c: self.a * q.c - self.b * q.d + self.c * q.a + self.d * q.b,
            d: self.a * q.d + self.b * q.c - self.c * q.b + self.d * q.a)
    }
}

extend Quaternion: Negatable {
    type Output = Quaternion;
    consuming func negate() -> Quaternion { Quaternion(a: -self.a, b: -self.b, c: -self.c, d: -self.d) }
}

extend Quaternion: Equatable {
    func isEqual(to q: Quaternion) -> Bool {
        self.a == q.a and self.b == q.b and self.c == q.c and self.d == q.d
    }
}

extend Quaternion: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        writer.append("(\(self.a), \(self.b), \(self.c), \(self.d))");
    }
}

@main
func main() {
    let q = Quaternion(a: 1.0, b: 2.0, c: 3.0, d: 4.0);
    let q1 = Quaternion(a: 2.0, b: 3.0, c: 4.0, d: 5.0);
    let q2 = Quaternion(a: 3.0, b: 4.0, c: 5.0, d: 6.0);
    let r = 7.0;
    println("q = \(q), q1 = \(q1), q2 = \(q2)");
    println("norm(q) = \(q.norm())");
    println("-q = \(-q)");
    println("conjugate(q) = \(q.conjugate())");
    println("q + r = \(q.plus(r))");
    println("q1 + q2 = \(q1 + q2)");
    println("q * r = \(q.times(r))");
    println("q1 * q2 = \(q1 * q2)");
    println("q2 * q1 = \(q2 * q1)");
    println("q1 * q2 == q2 * q1: \(q1 * q2 == q2 * q1)");
}
