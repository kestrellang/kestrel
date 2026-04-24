// test: diagnostics
// stdlib: false
module Test

protocol Mapper {
    type Source;
    func map(s: Source)
}
struct Box[T] { var value: T }
extend Box[T] where T: Mapper {
    func doMap(s: T.Source) {
        self.value.map(s)
    }
}
