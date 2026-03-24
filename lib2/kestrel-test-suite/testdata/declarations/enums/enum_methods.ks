// test: diagnostics
// stdlib: false

module Test

indirect enum LinkedList[T] {
    case Node(value: T, next: LinkedList[T])
    case Empty

    func length() -> lang.i64 { return 0; }
    static func createEmpty() -> LinkedList[T] { return .Empty; }
}
