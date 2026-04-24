// test: diagnostics
// stdlib: false
module Test

protocol Base {
    func baseMethod()
}
protocol Derived: Base {
    func derivedMethod()
}
extend Derived {
    func helper() {
        self.derivedMethod();
    }
}
struct Impl: Base, Derived {
    func baseMethod() { }
    func derivedMethod() { }
}
func test() {
    let i = Impl();
    i.helper();
}
