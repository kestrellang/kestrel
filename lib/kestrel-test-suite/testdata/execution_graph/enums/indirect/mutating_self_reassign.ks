// test: execution
// stdlib: true
// expect-exit: 0
//
// A `mutating func` that rebuilds `self` around its old value.

module Test

import std.numeric.(Int64)

indirect enum Stack {
    case Empty
    case Push(Int64, Stack)

    mutating func push(value: Int64) {
        self = .Push(value, self);
    }

    func count() -> Int64 {
        match self {
            .Empty => 0,
            .Push(_, rest) => 1 + rest.count()
        }
    }

    func top() -> Int64 {
        match self {
            .Empty => -1,
            .Push(value, _) => value
        }
    }
}

@main
func main() -> Int64 {
    var stack = Stack.Empty;
    stack.push(1);
    stack.push(2);
    stack.push(3);
    if stack.count() != 3 { return 1 }
    if stack.top() != 3 { return 2 }
    0
}
