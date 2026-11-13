// test: execution
// stdlib: true
// expect-exit: 0

// After `extend`, `[…]` is a param list only when a type follows its `]`;
// otherwise it is array (or dictionary) sugar for the target. `[T]?` used
// to be read as a param list followed by a bad target.

module Test

import std.numeric.Int64

protocol Tag {
    func tag() -> Int64
}

extend [Int64]?: Tag {
    func tag() -> Int64 { 1 }
}

extend [String]: Tag {
    func tag() -> Int64 { 2 }
}

extend [String: Int64]: Tag {
    func tag() -> Int64 { 3 }
}

extend [Int64] {
    func plain() -> Int64 { 4 }
}

extend Int64: Tag {
    func tag() -> Int64 { 5 }
}

@main
func main() -> lang.i32 {
    let maybe: [Int64]? = .Some([1]);
    if maybe.tag() != 1 { return 10 }
    let words: [String] = ["a"];
    if words.tag() != 2 { return 11 }
    let counts: [String: Int64] = ["a": 1];
    if counts.tag() != 3 { return 12 }
    let nums: [Int64] = [1];
    if nums.plain() != 4 { return 13 }
    0
}
