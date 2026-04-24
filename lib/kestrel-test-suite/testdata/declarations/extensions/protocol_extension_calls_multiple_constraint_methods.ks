// test: diagnostics
// stdlib: false
module Test

protocol A {
    func doA()
}
protocol B {
    func doB()
}
protocol C {
    func doC()
}
extend C where Self: A, Self: B {
    func doAll() {
        self.doC();
        self.doA();
        self.doB();
    }
}
struct Data: A, B, C {
    func doA() { }
    func doB() { }
    func doC() { }
}
func test() {
    let d = Data();
    d.doAll();
}
