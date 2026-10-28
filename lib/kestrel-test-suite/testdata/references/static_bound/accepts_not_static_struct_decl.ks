// test: execution
// backends: cranelift,llvm
// stdlib: true

// `Static` is an implicit-conformance builtin (references 2a), so a struct
// may declare itself reference-bearing with `: not Static` — the 2a test
// surface for non-Static types before refs can occupy fields (2b).

module Test

struct Handle: not Static {
    var id: Int64
}

@main
func main() {
    let h = Handle(id: 7);
    if h.id != 7 {
        fatalError("not-Static struct should still construct and read");
    }
    print("ok");
}

// CHECK: ok
