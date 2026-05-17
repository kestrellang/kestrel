// test: diagnostics
// stdlib: false
module Test

protocol Printable {
    func print()
}
extend Printable {
    func helper() { }
    func printTwice() {
        self.print();
        self.helper();
        self.print();
    }
}
struct Message: Printable {
    func print() { }
}
func test() {
    let m = Message();
    m.printTwice();
}
