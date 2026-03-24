// test: diagnostics
// stdlib: false
module Test

protocol Base {
    func base()
}
protocol Derived: Base {
    func derived()
}
extend Derived {
    func helper() { }
}
