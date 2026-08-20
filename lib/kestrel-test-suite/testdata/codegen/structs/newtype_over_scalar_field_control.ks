// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 control — pins the SCALAR-delegation branch that must NOT change.
//
// A one-field struct over a scalar field keeps its field's repr (an f64 newtype
// stays f64, an i32 newtype stays i32) rather than collapsing by byte size.
// That branch is load-bearing: mapping `Float64` to I64-by-size once made the
// auto clone-shim's signature disagree with its body and cranelift's verifier
// rejected the function. The F40 fix only added the aggregate-field arm beneath
// it; if a later cleanup "simplifies" the delegation away, this file fails.
module Test

struct WrapI64 {
    let n: std.numeric.Int64
}

struct WrapI32 {
    let n: std.numeric.Int32
}

struct WrapF64 {
    let f: std.numeric.Float64
}

struct WrapBool {
    let b: std.core.Bool
}

func makeI64(n: std.numeric.Int64) -> WrapI64 { WrapI64(n: n) }
func makeF64(f: std.numeric.Float64) -> WrapF64 { WrapF64(f: f) }

@main
func main() -> lang.i64 {
    var xs = std.collections.Array[WrapI64]();
    xs.append(makeI64(11));
    xs.append(makeI64(22));
    if xs(0).n != 11 { return 1 }
    if xs(1).n != 22 { return 2 }

    let a: std.numeric.Int32 = 7;
    if WrapI32(n: a).n != a { return 3 }

    // Float delegation: the value must survive as an f64, not an i64 bit-blob.
    let f = makeF64(2.5);
    let g = makeF64(4.0);
    if f.f + g.f != 6.5 { return 4 }

    // Cloning goes through the synthesized shim, whose signature reads the
    // same repr the body builds.
    let h = f;
    if h.f != 2.5 { return 5 }

    if not WrapBool(b: true).b { return 6 }
    0
}
