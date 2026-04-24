// test: diagnostics
// stdlib: false

module Main

struct Vector {
    var x: lang.i64
    var y: lang.i64

    func add((dx, dy): (lang.i64, lang.i64)) -> Vector {
        Vector(x: lang.i64_add(self.x, dx), y: lang.i64_add(self.y, dy))
    }
}

func test() -> lang.i64 {
    let v = Vector(x: 1, y: 2);
    let v2 = v.add((10, 20));
    v2.x
}
