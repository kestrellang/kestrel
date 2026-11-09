// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: spare 8\ncount 3: 10 11 12\ncopy kept 0\nreadAll 20005 last 5\n

// `spareCapacity()` lends the slots past `count`; `setCount` then makes the
// ones written there part of the array. Both unshare the buffer first, so a
// copy taken before is untouched. `readAll` uses them to read straight into
// the array, across many reads and growths.
module Test

import std.collections.(Slice)
import std.streams.(Readable, IoError, Cursor)

// Hands out at most 7 bytes per read, so `readAll` loops many times.
struct Trickle: Readable {
    var inner: Cursor

    mutating func read(into buf: &mutating Slice[UInt8]) -> Result[Int64, IoError] {
        let n = if buf.count < 7 { buf.count } else { 7 };
        self.inner.read(into: buf(0..<n))
    }
}

@main
func main() {
    var xs = Array[Int64](capacity: 8);
    let spare = unsafe xs.spareCapacity();
    print("spare \(spare.count)\n");
    for i in 0..<3 { spare(i) = 10 + i; }
    unsafe xs.setCount(3);
    print("count \(xs.count): \(xs(0)) \(xs(1)) \(xs(2))\n");

    let copy = xs;
    let more = unsafe xs.spareCapacity();
    more(0) = 99;
    unsafe xs.setCount(4);
    print("copy kept \(copy.count - 3)\n");

    var bytes = Array[UInt8]();
    for i in 0..<20000 { let b: UInt8 = UInt8(truncating: i % 251); bytes.append(b); }
    var src = Trickle(inner: Cursor(bytes: bytes));
    var dest = Array[UInt8]();
    for b in [1, 2, 3, 4, 5] { let v: UInt8 = UInt8(truncating: b); dest.append(v); }
    let n = match src.readAll(into: dest) { .Ok(n) => n, .Err(_) => -1 };
    var same = n == 20000;
    for i in 0..<20000 { if dest(5 + i) != bytes(i) { same = false; } }
    if not same { print("readAll wrong\n"); }
    print("readAll \(dest.count) last \(dest(4))\n");
}
