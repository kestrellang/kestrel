// Rosetta Code: Polymorphism
// https://rosettacode.org/wiki/Polymorphism
module Main

protocol Shape {
    func area() -> Float64
    func describe() -> String
}

struct Circle {
    let radius: Float64;
}

struct Rectangle {
    let width: Float64;
    let height: Float64;
}

extend Circle: Shape {
    func area() -> Float64 { Float64.pi * self.radius * self.radius }
    func describe() -> String { "circle of radius \(self.radius)" }
}

extend Rectangle: Shape {
    func area() -> Float64 { self.width * self.height }
    func describe() -> String { "\(self.width) x \(self.height) rectangle" }
}

// Static dispatch through a generic...
func report[S](shape: S) where S: Shape {
    println("\(shape.describe()): area \(shape.area():.3)");
}

@main
func main() {
    report(Circle(radius: 1.5));
    report(Rectangle(width: 2.0, height: 3.5));
    // ...or dynamic dispatch through an existential.
    let shapes: Array[any Shape] = [Circle(radius: 1.0), Rectangle(width: 4.0, height: 0.5)];
    let total = shapes.iter().map { it.area() }.sum();
    println("total area of \(shapes.count) shapes: \(total:.3)");
}
