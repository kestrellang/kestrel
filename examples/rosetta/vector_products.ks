// Rosetta Code: Vector products
// https://rosettacode.org/wiki/Vector_products
module Main

struct Vector3 {
    let x: Float64;
    let y: Float64;
    let z: Float64;

    func dot(other: Vector3) -> Float64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    func cross(other: Vector3) -> Vector3 {
        Vector3(x: self.y * other.z - self.z * other.y,
                y: self.z * other.x - self.x * other.z,
                z: self.x * other.y - self.y * other.x)
    }
}

extend Vector3: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        writer.append("(\(self.x), \(self.y), \(self.z))");
    }
}

@main
func main() {
    let a = Vector3(x: 3.0, y: 4.0, z: 5.0);
    let b = Vector3(x: 4.0, y: 3.0, z: 5.0);
    let c = Vector3(x: -5.0, y: -12.0, z: -13.0);
    println("a . b = \(a.dot(b))");
    println("a x b = \(a.cross(b))");
    println("a . (b x c) = \(a.dot(b.cross(c)))");
    println("a x (b x c) = \(a.cross(b.cross(c)))");
}
