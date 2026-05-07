// test: diagnostics
// stdlib: false

module Test

indirect enum LinkedList[T] { // ERROR: indirect enums are not yet supported
    case Node(value: T, next: LinkedList[T])
    case Empty

    func length() -> lang.i64 { return 0; }
    static func createEmpty() -> LinkedList[T] { return .Empty; }
}
