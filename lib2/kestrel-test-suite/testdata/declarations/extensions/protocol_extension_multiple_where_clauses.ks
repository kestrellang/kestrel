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
extend C where Self: A, Self: B {
    func helperAB() { }
}
