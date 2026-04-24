// test: diagnostics
// stdlib: false
module Test

protocol Processor {
    func process()
    func getState()
}
extend Processor {
    func processAndGetState() {
        self.process();
        let _state = self.getState();
    }
}
struct Item: Processor {
    func process() { }
    func getState() { }
}
func test() {
    let s = Item();
    s.processAndGetState();
}
