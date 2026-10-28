// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: start 0 now 3\nstart 3 now 5\n

// Copy -> borrow must see every name of the storage it borrows. In a
// `mutating` method, `let start = self.pos` reads through one `field_addr self`
// while `self.pos = ..` writes through another; turning the copy into a borrow
// would make `start` track the write. (Regression: LinesIterator.next.)
module Test

struct Cursor {
    var pos: Int64;
    var label: String;

    mutating func advance(by n: Int64) -> Int64 {
        let start = self.pos;
        self.pos = self.pos + n;
        println("start \(start) now \(self.pos)");
        start
    }
}

@main
func main() -> lang.i64 {
    var c = Cursor(pos: 0, label: "c");
    let a = c.advance(by: 3);
    let b = c.advance(by: 2);
    if a != 0 { return 1 }
    if b != 3 { return 2 }
    0
}
