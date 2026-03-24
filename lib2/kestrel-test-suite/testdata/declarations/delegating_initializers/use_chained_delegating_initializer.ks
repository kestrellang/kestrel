// test: diagnostics
// stdlib: false

module Test

struct Vector {
    var x: lang.i64
    var y: lang.i64
    var z: lang.i64

    init(x: lang.i64, y: lang.i64, z: lang.i64) {
        self.x = x;
        self.y = y;
        self.z = z
    }

    init(x: lang.i64, y: lang.i64) {
        self.init(x, y, 0)
    }

    init(x: lang.i64) {
        self.init(x, 0)
    }

    init() {
        self.init(0)
    }
}

func test() -> (Vector, Vector, Vector, Vector) {
    (Vector(1, 2, 3),
     Vector(1, 2),
     Vector(1),
     Vector())
}
