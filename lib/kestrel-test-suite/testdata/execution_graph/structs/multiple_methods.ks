// test: diagnostics
// stdlib: false

module Main

struct Rectangle {
    let width: lang.i64
    let height: lang.i64
    
    func area() -> lang.i64 {
        lang.i64_mul(self.width, self.height)
    }

    func perimeter() -> lang.i64 {
        lang.i64_mul(2, lang.i64_add(self.width, self.height))
    }
}
