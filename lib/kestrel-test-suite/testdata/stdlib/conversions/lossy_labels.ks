// test: execution
// stdlib: true

module Test

// Integer conversions: `from:` is exact, `truncating:` keeps the low bits,
// `clamping:` saturates, and `bitPattern:` reinterprets a same-width value.
@main
func main() -> lang.i64 {
    let big: Int64 = 300;
    if Int8(truncating: big) != 44 { return 1 }
    if Int8(clamping: big) != 127 { return 2 }
    if Int8(clamping: -big) != -128 { return 3 }
    if UInt8(clamping: -big) != 0 { return 4 }
    let ok: Int64 = 100;
    if Int8(from: ok) != 100 { return 5 }
    let neg: Int32 = -1;
    if UInt32(bitPattern: neg) != 4294967295 { return 6 }
    if UInt64(truncating: neg) != 18446744073709551615 { return 7 }
    if UInt64(clamping: neg) != 0 { return 8 }
    let max: UInt64 = 18446744073709551615;
    if Int64(clamping: max) != Int64.maxValue { return 9 }
    if Int64(bitPattern: max) != -1 { return 10 }
    // Widening stays a plain, non-trapping `from:`.
    if Int64(from: neg) != -1 { return 11 }
    0
}
