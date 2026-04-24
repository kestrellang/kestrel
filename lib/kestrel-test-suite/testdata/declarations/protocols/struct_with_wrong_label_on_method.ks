// test: diagnostics
// stdlib: false
module Test
protocol Greetable {
    func greet(with name: lang.str)
}
struct Person: Greetable { // ERROR: does not implement method 'greet'
    func greet(using name: lang.str) { }
}
