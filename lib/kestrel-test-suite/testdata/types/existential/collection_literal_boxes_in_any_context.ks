// test: execution
// stdlib: true
// expect-stdout: 10\n10\n12\n24 2\n2\n

// A collection literal checked against `Array[any P]` or `[K: any P]` boxes
// each element, wherever the expected type comes from: a call argument, a
// struct field, a method argument, an assignment or a return.
module Test

protocol Shape { func area() -> Int64 }
struct Sq: Shape { func area() -> Int64 { 4 } }
struct Rc: Shape { func area() -> Int64 { 6 } }

func total(shapes: Array[any Shape]) -> Int64 {
    var t = 0;
    for s in shapes { t += s.area() }
    t
}

func make() -> Array[any Shape] { [Sq(), Rc()] }

struct Scene {
    var shapes: Array[any Shape]
    mutating func add(all more: Array[any Shape]) { for s in more { self.shapes.append(s) } }
}

@main
func main() -> Int64 {
    let a: Array[any Shape] = [Sq(), Rc()];
    println("\(total(a))");
    println("\(total([Sq(), Rc()]))");
    var b: Array[any Shape] = [];
    b = [Rc(), Rc()];
    println("\(total(b))");
    var sc = Scene(shapes: [Sq(), Rc()]);
    sc.add(all: [Rc(), Sq(), Sq()]);
    println("\(total(sc.shapes)) \(make().count)");
    var d: [String: any Shape] = [:];
    d = ["a": Sq(), "b": Rc()];
    println("\(d.count)");
    0
}
