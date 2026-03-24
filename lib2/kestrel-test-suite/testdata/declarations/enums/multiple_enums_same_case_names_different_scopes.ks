// test: diagnostics
// stdlib: false
module Test
enum A {
    case Value
}

enum B {
    case Value
}

func test() {
    let a: A = .Value;
    let b: B = .Value;
}
