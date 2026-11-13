// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 7

// Known bug: a `&xs(i)` binding inside a closure over a captured array
// fails OSSA verification ("operand ... used but never defined"). The same
// binding outside a closure works. Found drawing chunk meshes inside a
// wgpu render-pass closure (examples/aerie/src/renderer.ks).

module Test

func each(body: (Int64) -> ()) {
    body(0);
}

@main
func main() -> Int64 {
    let xs = [7];
    each { (i) in
        let h = &xs(i);
        if h == 7 { println("seen"); }
    };
    xs(0)
}
