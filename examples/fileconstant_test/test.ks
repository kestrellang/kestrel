// Test for @fileconstant attribute
// Minimal no-std test

module FileConstantTest

// Minimal LiteralSlice definition (no std)
struct LiteralSlice[T] {
    var ptr: lang.ptr[T]
    var len: lang.i64
}

@fileconstant("test_data.bin")
let TEST_DATA: LiteralSlice[lang.i64]

func main() {
    // Access the slice - this tests that the file constant compiles
    let slice = TEST_DATA;
    let _len = slice.len;
}
