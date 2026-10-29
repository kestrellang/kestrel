// test: execution
// stdlib: true
// expect-exit: 0

// Omitted `some P` defaults on a method and a static method (the shape of
// `ChaCha20Poly1305.seal(message, using:, authenticating: = [], nonce: = …)`):
// the opaque type comes from the default expression, also when a later
// defaulted argument is given and the opaque one skipped.

module Test
import std.numeric.(Int64, UInt8)
struct Summer {
    var base: Int64
    func total(data: some Slice[UInt8], extra extra: some Slice[UInt8] = Array[UInt8]()) -> Int64 {
        var sum: Int64 = self.base;
        for b in data.asSlice() { sum = sum + Int64(from: b); }
        for b in extra.asSlice() { sum = sum + Int64(from: b); }
        sum
    }
    static func seal(data: some Slice[UInt8], aad aad: some Slice[UInt8] = Array[UInt8](), nonce nonce: Int64 = 5) -> Int64 {
        var sum: Int64 = nonce;
        for b in data.asSlice() { sum = sum + Int64(from: b); }
        for b in aad.asSlice() { sum = sum + Int64(from: b); }
        sum
    }
}
@main
func main() -> lang.i64 {
    let a: [UInt8] = [1, 2, 3];
    let s = Summer(base: 10);
    if s.total(a) != 16 { return 1 }
    if s.total(a, extra: a) != 22 { return 2 }
    if Summer.seal(a) != 11 { return 3 }
    if Summer.seal(a, nonce: 0) != 6 { return 4 }
    if Summer.seal(a, aad: a) != 17 { return 5 }
    0
}
