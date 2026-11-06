// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 40 20 8 4\n

// The size of a slice payload is its count times the element size; its
// alignment is the element's. Word 1 of a slice's fat pointer is the count,
// not a function table.
module Test
import std.collections.(Array, Slice)

@main
func main() -> Int64 {
    var xs: Array[Int64] = [1, 2, 3, 4, 5];
    let fat: lang.ptr[lang.slice[Int64]] = lang.slice_from_parts(xs.asUnsafeSlice().pointer.raw, 5);
    let whole: lang.ptr[Slice[Int64]] = lang.cast_ptr[_, Slice[Int64]](fat);
    let narrow: lang.ptr[lang.slice[Int32]] = lang.cast_ptr[_, lang.slice[Int32]](fat);
    let a = Int64(raw: lang.sizeof_val[Slice[Int64]](whole));
    let b = Int64(raw: lang.sizeof_val[lang.slice[Int32]](narrow));
    let c = Int64(raw: lang.alignof_val[Slice[Int64]](whole));
    let d = Int64(raw: lang.alignof_val[lang.slice[Int32]](narrow));
    print("\(a) \(b) \(c) \(d)\n");
    0
}
