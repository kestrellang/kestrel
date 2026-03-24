// test: diagnostics
// stdlib: false
module Test

protocol Builder {
    func reset()
    func validate()
}
extend Builder {
    func prepareAndValidate() {
        self.reset();
        self.validate();
        self.reset();
        self.validate();
    }
}
struct SimpleBuilder: Builder {
    func reset() { }
    func validate() { }
}
func test() {
    let b = SimpleBuilder();
    b.prepareAndValidate();
}
