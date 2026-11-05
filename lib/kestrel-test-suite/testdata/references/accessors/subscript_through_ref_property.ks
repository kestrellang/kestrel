// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: read 2 2 5 4 write 77 87 a 15\n

// A subscript applied to a reference-typed computed property
// (`p.value(i)`, `p.mutatingValue(i) = v`) reads and writes the referent.
// The read used to leave the property's type unsubstituted (a post-mono
// ICE on a generic owner like `Pointer[T]`), and the write was dropped
// without a trace.
module Test
import std.memory.(Pointer)

struct Pair { var a: Int64; var b: Int64 }

extend Pair {
    subscript(i: Int64) -> Int64 {
        ref { self.b }
        mutating ref { self.b }
    }
}

@main
func main() -> Int64 {
    var x = Pair(a: 1, b: 2);
    let p = Pointer(to: x);
    var xs: Array[Int64] = [4, 5];
    let q = Pointer(to: xs);
    print("read \(p.value(1)) \(p.mutatingValue(1)) \(q.value(1)) \(q.value(0)) ");
    p.mutatingValue(1) = 77;
    print("write \(x.b) ");
    let m = &mutating unsafe p.mutatingValue;
    m(1) += 10;
    unsafe p.mutatingValue.a = 5;
    m.a += 10;
    print("\(x.b) a \(x.a)\n");
    0
}
