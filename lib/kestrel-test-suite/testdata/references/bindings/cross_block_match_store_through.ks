// test: execution
// backends: cranelift,llvm
// stdlib: true

// A `&mutating` binding crosses match decision-tree blocks and arms;
// store-through inside an arm writes the original var.

module Test

enum Cmd {
    case Add(Int64)
    case Reset
}

@main
func main() {
    var x = 5;
    let m = &mutating x;
    let cmd = Cmd.Add(3);
    match cmd {
        .Add(n) => { m = x + n; },
        .Reset => { m = 0; },
    }
    if x != 8 {
        fatalError("store-through across match arms broke: \(x)");
    }
    m = m + 1;
    if x != 9 {
        fatalError("post-match store-through broke: \(x)");
    }
    print("ok");
}

// CHECK: ok
