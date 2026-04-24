// test: diagnostics
// stdlib: false

module Main

struct Point3D {
    var x: lang.i64
    var y: lang.i64
    var z: lang.i64
}

func test(p: Point3D) -> lang.i64 {
    match p {
        Point3D { x, .. } => x
    }
}
