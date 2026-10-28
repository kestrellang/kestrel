// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b: a struct stores a ref FIELD (memberwise init; the struct
// is structurally non-Static). Reads go through the loaded ref across a
// loop; may-alias writes to the source stay visible.
module Test

struct Cursor {
    var item: &Int64
    var count: Int64
}

@main
func main() {
    var x = 10;
    let r = &x;
    let c = Cursor(item: r, count: 3);
    var total = 0;
    var i = 0;
    while i < c.count {
        total = total + c.item;
        i = i + 1;
    }
    x = 100;
    total = total + c.item;
    if total != 130 {
        fatalError("ref field reads broke: \(total)");
    }
    print("ok");
}

// CHECK: ok
