// test: execution
// stdlib: true
// expect-exit: 0

// An opaque `some P` parameter with a default can be omitted: its type is
// the default expression's (`Array[UInt8]` here), as Swift's SE-0347 infers
// a generic parameter from its default. It used to fail with E107 "does not
// satisfy constraint `_ !: Slice[UInt8]`" — nothing pinned the parameter.

module Test

func total(data: some Slice[UInt8], extra extra: some Slice[UInt8] = Array[UInt8]()) -> Int64 {
    var sum: Int64 = 0;
    for b in data.asSlice() { sum = sum + Int64(from: b); }
    for b in extra.asSlice() { sum = sum + Int64(from: b); }
    sum
}

@main
func main() -> lang.i64 {
    let a: [UInt8] = [1, 2, 3];

    // default omitted (the gap): extra is the empty Array[UInt8].
    if total(a) != 6 { return 1 }

    // default provided (works today).
    if total(a, extra: a) != 12 { return 2 }

    0
}
