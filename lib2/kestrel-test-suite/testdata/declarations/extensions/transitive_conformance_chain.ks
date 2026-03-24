// test: diagnostics
// stdlib: false
module Test

protocol D {
    func methodD()
}
protocol C {
    func methodC()
}
protocol B {
    func methodB()
}
extend B: C {
    func methodC() { }
}
extend C: D {
    func methodD() { }
}
struct A: B {
    func methodB() { }
}
func requiresD[T](x: T) where T: D {
    x.methodD();
}
func test() {
    let a = A();
    requiresD(a);
}
