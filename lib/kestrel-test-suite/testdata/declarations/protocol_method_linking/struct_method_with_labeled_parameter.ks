// test: diagnostics
// stdlib: false
module Test

protocol Greetable {
    func greet(with name: lang.str)
}
struct Person: Greetable {
    func greet(with name: lang.str) { }
}
