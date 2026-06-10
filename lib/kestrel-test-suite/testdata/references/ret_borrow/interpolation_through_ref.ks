// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: v=42

// String interpolation is receiver see-through: the Formattable dispatch
// peels the ref and formats the pointee in place.
module Test

@main
func main() -> lang.i64 {
    let arr = [42];
    print("v=\(arr.at(index: 0))");
    0
}
