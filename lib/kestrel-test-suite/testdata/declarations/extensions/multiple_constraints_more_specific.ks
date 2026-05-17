// test: diagnostics
// stdlib: false
module Test

protocol A {
    func methodA()
}
protocol B {
    func methodB()
}
protocol C {
    func methodC()
}
// Specificity 1 (one constraint)
extend C where Self: A {
    func helper() { }
}
// Specificity 2 (two constraints) - should win
extend C where Self: A, Self: B {
    func helper() { }
}
struct Data: A, B, C {
    func methodA() { }
    func methodB() { }
    func methodC() { }
}
func test() {
    let d = Data();
    d.helper();
}
