// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
    
    func sum() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}

func main() -> lang.i64 {
    let p = Point(x: 3, y: 4);
    p.sum()
}
