// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b: tuples carry ref elements when the TYPE pins them;
// extraction loads each stored address and both refs alias their sources.
module Test

@main
func main() {
    var a = 1;
    var b = 2;
    let ra = &a;
    let rb = &b;
    let t: (&Int64, &Int64, Int64) = (ra, rb, 10);
    a = 5;
    b = 7;
    let total = t.0 + t.1 + t.2;
    if total != 22 {
        fatalError("ref tuple elements broke: \(total)");
    }
    print("ok");
}

// CHECK: ok
