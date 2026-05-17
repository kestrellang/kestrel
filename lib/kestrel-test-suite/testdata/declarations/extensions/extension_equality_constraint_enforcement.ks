// test: diagnostics
// stdlib: false
module Test

protocol Mapper {
    type Source;
    func map(s: Source)
}
struct Box[T] { var value: T }
extend Box[T] where T: Mapper, T.Source = lang.i64 {
    func mapString(s: lang.str) {
        // Should fail: T.Source is lang.i64, but s is String
        self.value.map(s) // ERROR: type mismatch
    }
}
