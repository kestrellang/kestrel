// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: in unsafe\ndrop inner\nafter unsafe\ndrop a\nkept b\ndrop loop 0\ndrop loop 1\ndrop loop 2\ndrop early\nearly 1\ndrop early\nnot early\nearly 2\nend\ndrop b\ndrop outer\n
//
// `unsafe { … }` is a drop scope of its own: a local declared in it drops at
// its `}`, not at the end of the function. Its value moves out. Early exits
// (`return`, `break`, `continue`) drop the block's locals on the way out.

module Test

struct Noisy: not Copyable {
    let name: String
    deinit { println("drop \(self.name)"); }
}

func early(flag: Bool) -> Int64 {
    var x: Int64 = 0;
    let p = Pointer(to: x);
    unsafe {
        let n = Noisy(name: "early");
        p.write(1);
        if flag { return p.read(); }
    };
    println("not early");
    2
}

@main
func main() {
    var x: Int64 = 0;
    let p = Pointer(to: x);
    let outer = Noisy(name: "outer");
    unsafe {
        let n = Noisy(name: "inner");
        p.write(1);
        println("in unsafe");
    };
    println("after unsafe");
    let kept = unsafe {
        let a = Noisy(name: "a");
        let b = Noisy(name: "b");
        p.write(2);
        b
    };
    println("kept \(kept.name)");
    var i: Int64 = 0;
    while i < 3 {
        unsafe {
            let l = Noisy(name: "loop \(i)");
            p.write(i);
            if i == 1 { i = i + 1; continue; }
            if i == 2 { break; }
        };
        i = i + 1;
    }
    println("early \(early(true))");
    println("early \(early(false))");
    println("end");
}
