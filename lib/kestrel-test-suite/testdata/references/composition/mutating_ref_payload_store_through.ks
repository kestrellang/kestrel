// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b: &mutating payloads — the extracted binding writes
// through to the original storage.
module Test

@main
func main() {
    var x = 1;
    let r = &mutating x;
    let o: Optional[&mutating Int64] = .Some(r);
    if let .Some(m) = o {
        m = 42;
    } else {
        fatalError("Some matched as None");
    }
    if x != 42 {
        fatalError("store through extracted &mutating payload lost: \(x)");
    }
    print("ok");
}

// CHECK: ok
