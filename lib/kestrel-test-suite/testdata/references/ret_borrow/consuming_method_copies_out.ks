// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `consuming` method is NOT a place context: called through a ref it
// reads the place (clone) and consumes the COPY — the original stays
// alive. Detector: the copy's deinit decrements the shared cell exactly
// once inside the scope; the original decrements at scope exit.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Item: Cloneable {
    var name: String
    var cell: Pointer[Int64]
    func clone() -> Item {
        // payload-clone, never `{ self }` (the aliasing footgun)
        Item(name: self.name.clone(), cell: self.cell)
    }
    deinit { self.cell.write(self.cell.read() - 1); }

    func tag() -> String { self.name }
    consuming func consume() -> Int64 { 7 }
}

struct Box {
    var item: Item
    func peek() -> &Item { self.item }
}

func run(counter: Pointer[Int64]) -> Int64 {
    let b = Box(item: Item(name: "x", cell: counter));
    if b.peek().consume() != 7 { return 1; }
    // exactly the copy deinit'd so far: 10 - 1 == 9
    if counter.read() != 9 { return 2; }
    // the original is alive and untouched
    if b.peek().tag() != "x" { return 3; }
    0
}

@main
func main() -> lang.i64 {
    let counter = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    counter.write(10);

    let status = run(counter);
    if status != 0 { return status.raw; }

    // the original deinit'd at `run` scope exit: 9 - 1 == 8
    if counter.read() != 8 { return 4; }
    0
}
