// test: diagnostics
// stdlib: false
module Test
enum Shape {
    case Circle(radius: lang.f64)
}

func test() {
    let x = .Circle(radius: 5.0); // ERROR: implicit member '.Circle' not found
}
