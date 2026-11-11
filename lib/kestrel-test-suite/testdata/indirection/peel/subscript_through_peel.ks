// test: execution
// stdlib: true
// expect-stdout: 2 9 13 2 20\n

// A subscript reached through the peel (the wrapper declares none) runs on
// the target: reads lend through `value`'s `ref`, and assignment and
// compound assignment through its `mutating ref`. The callee's type args
// come from the target, not the wrapper. Both used to break: reads ICEd in
// post-mono verify, writes reached the wrong receiver and panicked.
module Test

import std.collections.(Array, Slice)
import std.core.(Indirection)
import std.memory.(OwnedBox)

struct View { let base: &Array[Int64] }

extend View: Indirection {
    type Target = Slice[Int64]
    var value: Slice[Int64] { ref { self.base.asSlice() } }
}

@main
func main() {
    var b = OwnedBox([1, 2, 3]);
    let x = b(1);
    b(0) = 9;
    b(2) += 10;
    let xs = [10, 20, 30];
    let r = &xs;
    let v = View(base: r);
    println("\(x) \(b(0)) \(b(2)) \(b(0..<2).count) \(v(1))");
}
