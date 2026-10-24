// test: diagnostics
// stdlib: false
//
// E912: nothing may hold the address of a packed field, which may be
// misaligned. Reads, whole-field writes, plain (copying) arguments and
// borrows of the whole struct are fine.

module Main

struct Counter {
    var n: lang.i64;
    mutating func bump() { self.n = lang.i64_add(self.n, 1); }
}

@layout(.Packed)
struct Header {
    var tag: lang.i8;
    var length: lang.i64;
    var counter: Counter;
    mutating func reset() { self.length = 0; }
}

func read(n: lang.i64) -> lang.i64 { n }
func bumpIt(mutating n: lang.i64) { n = lang.i64_add(n, 1); }
func touch(mutating h: Header) { h.length = 1; }

func uses() -> lang.i64 {
    var h = Header(tag: 1, length: 2, counter: Counter(n: 0));
    let r = &h.length; // ERROR: cannot borrow 'h.length' in place: 'Header' is packed, so the field may be misaligned
    let m = &mutating h.length; // ERROR: cannot borrow 'h.length' in place
    bumpIt(h.length); // ERROR: cannot borrow 'h.length' in place
    h.counter.bump(); // ERROR: cannot borrow 'h.counter' in place
    bumpIt(h.counter.n); // ERROR: cannot borrow 'h.counter.n' in place

    // Fine: reads, whole-field writes, copies, the whole struct.
    h.length = 5;
    let copy = read(h.length);
    var n = h.length;
    bumpIt(n);
    h.reset();
    touch(h);
    let whole = &h;
    copy
}
