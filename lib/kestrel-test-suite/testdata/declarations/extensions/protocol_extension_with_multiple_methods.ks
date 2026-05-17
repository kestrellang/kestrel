// test: diagnostics
// stdlib: false
module Test

protocol Processable {
    func process()
}
extend Processable {
    func helper1() { }
    func helper2() { }
    func helper3() { }
}
